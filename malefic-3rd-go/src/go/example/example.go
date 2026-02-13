package example

import "malefic-3rd-go/malefic"

// Module is the simplest GoModuleHandler implementation.
// Just implement Handle — no channels, no loops.
type Module struct{}

func (m *Module) Name() string { return "example_go" }

func (m *Module) Handle(taskId uint32, req *malefic.Request) (*malefic.Response, error) {
	return &malefic.Response{
		Output: "hello from golang module, input: " + req.Input,
	}, nil
}
