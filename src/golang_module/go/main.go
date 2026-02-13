package main

/*
#include <stdlib.h>
*/
import "C"
import (
	"malefic-3rd-go/example"
	"malefic-3rd-go/malefic"
	"sync"
	"unsafe"
)

// module is the singleton module instance.
// Switch to the implementation you need:
//
//	var module malefic.GoModule = malefic.AsModule(&example.Module{})    // simple handler
//	var module malefic.GoModule = &hackbrowser.Module                   // streaming
var module malefic.GoModule = malefic.AsModule(&example.Module{})

// session holds per-task channels for bidirectional streaming.
type session struct {
	input  chan *malefic.Request
	output chan *malefic.Response
	done   chan struct{}
}

var sessions sync.Map // map[uint32]*session

// getOrCreateSession returns the session for taskId, creating one on first access.
func getOrCreateSession(taskId uint32) *session {
	if v, ok := sessions.Load(taskId); ok {
		return v.(*session)
	}
	s := &session{
		input:  make(chan *malefic.Request, 16),
		output: make(chan *malefic.Response, 16),
		done:   make(chan struct{}),
	}
	actual, loaded := sessions.LoadOrStore(taskId, s)
	if loaded {
		return actual.(*session)
	}
	go func() {
		defer close(s.done)
		defer close(s.output)
		module.Run(taskId, s.input, s.output)
	}()
	return s
}

//export GoModuleName
func GoModuleName() *C.char {
	return C.CString(module.Name())
}

//export GoModuleSend
func GoModuleSend(taskId C.uint, data *C.char, dataLen C.int) C.int {
	s := getOrCreateSession(uint32(taskId))
	buf := C.GoBytes(unsafe.Pointer(data), dataLen)
	req := &malefic.Request{}
	if err := req.UnmarshalVT(buf); err != nil {
		return -1
	}
	select {
	case s.input <- req:
		return 0
	case <-s.done:
		return -1
	}
}

//export GoModuleRecv
func GoModuleRecv(taskId C.uint, outLen *C.int, status *C.int) *C.char {
	s := getOrCreateSession(uint32(taskId))
	resp, ok := <-s.output
	if !ok {
		sessions.Delete(uint32(taskId))
		*status = 1
		return nil
	}
	out, err := resp.MarshalVT()
	if err != nil {
		*status = 2
		return nil
	}
	*outLen = C.int(len(out))
	*status = 0
	return (*C.char)(C.CBytes(out))
}

//export GoModuleCloseInput
func GoModuleCloseInput(taskId C.uint) {
	v, ok := sessions.Load(uint32(taskId))
	if !ok {
		return
	}
	s := v.(*session)
	close(s.input)
}

//export GoFreeBuffer
func GoFreeBuffer(ptr *C.char) {
	C.free(unsafe.Pointer(ptr))
}

func main() {}
