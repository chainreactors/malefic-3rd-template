# Malefic 3rd Party Module Template

用于创建 malefic implant 第三方模块的模板项目。

Malefic 本体采用最小化依赖设计，所有需要引入第三方库的 module 都在 3rd 中实现。官方维护的公开 3rd module 见 [malefic-3rd](https://github.com/chainreactors/malefic/tree/master/malefic-3rd)。

## 项目结构

```
malefic-3rd-template/
├── Cargo.toml
├── build.rs                          # 自动编译 Go → c-archive 并链接
├── src/
│   ├── lib.rs                        # 模块注册入口
│   ├── prelude.rs
│   ├── rust_module/
│   │   └── mod.rs                    # Rust 模块示例
│   └── golang_module/
│       ├── mod.rs                    # Rust 侧 FFI 桥接 + 流式 run()
│       └── go/
│           ├── go.mod
│           ├── main.go               # FFI 导出 (Send/Recv) + session 管理
│           ├── malefic/
│           │   ├── module.go         # GoModule / GoModuleHandler 接口
│           │   ├── module.proto      # Request/Response 定义
│           │   └── module.pb.go      # protobuf 生成代码
│           ├── example/
│           │   └── example.go        # 示例 1: 简单 echo (GoModuleHandler)
│           └── hackbrowser/
│               └── hackbrowser.go    # 示例 2: HackBrowserData 流式 (GoModule)
└── tests/
    └── test_load_dll.rs              # 集成测试
```

## Feature 按需加载

通过 Cargo feature 控制编译哪些模块，未启用的模块不会编译进产物。

```toml
[features]
default = ["as_cdylib", "full"]

full = ["rust_module", "golang_module"]

rust_module = []
golang_module = ["prost"]       # Go 模块需要 prost 做 protobuf 编解码
```

### 选择性构建

```bash
# 全部模块
cargo build --target x86_64-pc-windows-gnu --release

# 只要 Rust 模块（不编译 Go）
cargo build --target x86_64-pc-windows-gnu --no-default-features \
  --features "as_cdylib,rust_module" --release

# 只要 Go 模块
cargo build --target x86_64-pc-windows-gnu --no-default-features \
  --features "as_cdylib,golang_module" --release
```

### 注册机制

`src/lib.rs` 中通过 `register_module!` 宏和 `#[cfg(feature)]` 实现按需注册：

```rust
#[cfg(feature = "rust_module")]
pub mod rust_module;
#[cfg(feature = "golang_module")]
pub mod golang_module;

pub extern "C" fn register_3rd() -> MaleficBundle {
    let mut map: MaleficBundle = HashMap::new();

    // Rust 模块：宏内部自带 #[cfg(feature = "...")]
    register_module!(map, "rust_module", rust_module::RustModule);

    // Go 模块：名称由 Go 侧运行时返回，需手动 cfg
    #[cfg(feature = "golang_module")]
    golang_module::register(&mut map);

    map
}
```

添加新模块只需三步：

1. `Cargo.toml` 加 feature（按需加入 `full`）
2. 写模块代码，mod 声明加 `#[cfg(feature = "xxx")]`
3. 注册处加一行 `register_module!`

## Rust 模块开发

```rust
use crate::prelude::*;

pub struct YourModule {}

#[async_trait]
#[module_impl("your_module")]
impl Module for YourModule {}

#[async_trait]
impl ModuleImpl for YourModule {
    async fn run(&mut self, id: u32, receiver: &mut Input, sender: &mut Output) -> ModuleResult {
        let request = check_request!(receiver, Body::Request)?;
        let response = Response {
            output: "hello".to_string(),
            ..Default::default()
        };
        Ok(TaskResult::new_with_body(id, Body::Response(response)))
    }
}
```

## Go 模块开发

### 架构

Rust 和 Go 之间通过双向流式 FFI 通信：

```
Rust async                          Go goroutine
─────────                          ─────────────
Input channel ──GoModuleSend()──→  input chan
                                       ↓
                                   module.Run()
                                       ↓
recv thread   ←─GoModuleRecv()───  output chan
    ↓
mpsc::unbounded
    ↓
futures::select! ──→ Output/return
```

核心 FFI 只有两个函数：

| 函数 | 说明 |
|------|------|
| `GoModuleSend(taskId, data, len)` | 发送请求。首次调用自动创建 session 并启动 goroutine |
| `GoModuleRecv(taskId, outLen, status)` | 阻塞读取响应。`status=1` 表示 Go 侧结束，session 自动清理 |

### 两层接口

Go 侧提供两层抽象，类似 Rust 侧的 `check_request!` 宏：

```go
// GoModuleHandler — 简单模块只需实现这个接口，无需接触 channel。
type GoModuleHandler interface {
    Name() string
    Handle(taskId uint32, req *malefic.Request) (*malefic.Response, error)
}

// GoModule — 底层流式接口，需要双向流（多响应/长任务）时直接实现。
type GoModule interface {
    Name() string
    Run(taskId uint32, input <-chan *malefic.Request, output chan<- *malefic.Response)
}
```

`malefic.AsModule(handler)` 可将 `GoModuleHandler` 包装为 `GoModule`，内部自动循环 input channel 并转发响应：

```go
import (
    "malefic-3rd-go/malefic"
    "malefic-3rd-go/example"
    // "malefic-3rd-go/hackbrowser"
)

var module malefic.GoModule = malefic.AsModule(&example.Module{})     // 简单模块用 AsModule 包装
var module malefic.GoModule = &hackbrowser.Module{}                   // 流式模块直接赋值
```

### Protobuf 协议

```protobuf
message Request {
  string name = 1;
  string input = 2;
  repeated string args = 3;
  map<string, string> params = 4;
  bytes bin = 5;
}

message Response {
  string output = 1;
  string error = 2;
  map<string, string> kv = 3;
  repeated string array = 4;
}
```

### 切换 Go 模块

编辑 `main.go` 中的 `module` 变量：

```go
var module malefic.GoModule = malefic.AsModule(&example.Module{})     // 简单模块
var module malefic.GoModule = &hackbrowser.Module{}                   // 流式模块
```

### 示例 1: Hello（GoModuleHandler）

最简单的模块，只需实现 `Handle`，无需接触 channel（`example/example.go`）：

```go
package example

import "malefic-3rd-go/malefic"

type Module struct{}

func (m *Module) Name() string { return "example_go" }

func (m *Module) Handle(taskId uint32, req *malefic.Request) (*malefic.Response, error) {
    return &malefic.Response{
        Output: "hello from golang module, input: " + req.Input,
    }, nil
}
```

### 示例 2: HackBrowserData（流式多响应）

集成 [HackBrowserData](https://github.com/moonD4rk/HackBrowserData)，直接实现 `GoModule` 接口，一个请求触发多个流式响应（`hackbrowser/hackbrowser.go`）：

```go
package hackbrowser

type Module struct

func (m *Module) Name() string { return "hack_browser_data" }

func (m *Module) Run(taskId uint32, input <-chan *malefic.Request, output chan<- *malefic.Response) {
    // 流式处理...
}
```

请求参数：

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `Request.Input` | 浏览器名 | `"all"` |
| `Request.Params["format"]` | 输出格式 | `"json"` |
| `Request.Params["profile_path"]` | 自定义 profile 路径 | 自动检测 |
| `Request.Params["full_export"]` | 是否完整导出 | `"true"` |

响应流：

1. 每个浏览器的每种数据类型（password、cookie、history…）各返回一个 `Response`，`kv` 中包含 `browser` 和 `file` 字段
2. 最后返回一个汇总 `Response`，`kv.status = "complete"`

### 编写自己的 Go 模块

1. 在 `go/` 下创建新目录（如 `go/yourmod/`），实现模块
   - 简单请求→响应：实现 `malefic.GoModuleHandler`，用 `malefic.AsModule()` 包装
   - 需要流式/多响应：直接实现 `malefic.GoModule`
2. 在 `go.mod` 中添加依赖（`go get ...`）
3. 修改 `main.go` 中 `module` 变量指向你的实现
4. 构建即可

## 构建与测试

```bash
# 构建
cargo build --target x86_64-pc-windows-gnu --features golang_module --release

# 测试
cargo test --target x86_64-pc-windows-gnu --features golang_module -- --nocapture

# 加载到 implant
load_module --path target/x86_64-pc-windows-gnu/release/malefic_3rd.dll
```
