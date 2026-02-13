# Malefic 3rd Party Module Template

用于创建 malefic implant 第三方模块的模板项目，支持 **Rust / Go / C / Zig / Nim** 五种语言编写模块。

Malefic 本体采用最小化依赖设计，所有需要引入第三方库的 module 都在 3rd 中实现。官方维护的公开 3rd module 见 [malefic-3rd](https://github.com/chainreactors/malefic/tree/master/malefic-3rd)。

## 项目结构

```
malefic-3rd-template/
├── Cargo.toml                        # Workspace 根 + cdylib 入口
├── src/lib.rs                        # 模块注册入口 (register_modules)
├── malefic-3rd-ffi/                  # 共享 FFI 工具库 (FfiBuffer, encode/decode)
├── malefic-3rd-rust/                 # Rust 模块
├── malefic-3rd-go/                   # Go 模块 (双向流式 FFI)
├── malefic-3rd-c/                    # C 模块 (nanopb + 同步 handler)
├── malefic-3rd-zig/                  # Zig 模块 (nanopb + 同步 handler)
├── malefic-3rd-nim/                  # Nim 模块 (nanopb + 同步 handler)
└── tests/test_load_dll.rs            # 集成测试 (动态加载 DLL 验证)
```

## Feature 按需加载

通过 Cargo feature 控制编译哪些模块，未启用的模块不会编译进产物。

```toml
[features]
full = ["rust_module", "golang_module", "c_module", "zig_module", "nim_module"]

rust_module   = ["malefic-3rd-rust"]
golang_module = ["malefic-3rd-go"]
c_module      = ["malefic-3rd-c"]
zig_module    = ["malefic-3rd-zig"]
nim_module    = ["malefic-3rd-nim"]
```

### 选择性构建

```bash
# 全部模块
cargo build --target x86_64-pc-windows-gnu --release

# 只要 Rust + Go
cargo build --target x86_64-pc-windows-gnu --no-default-features \
  --features "as_cdylib,rust_module,golang_module" --release

# 只要 C + Zig
cargo build --target x86_64-pc-windows-gnu --no-default-features \
  --features "as_cdylib,c_module,zig_module" --release
```

## FFI 协议

所有非 Rust 语言模块遵循相同的 C ABI 协议：

| 导出函数 | 签名 | 说明 |
|----------|------|------|
| `XxxModuleName()` | `() -> *const char` | 返回模块名（静态字符串，不需要 free） |
| `XxxModuleHandle()` | `(task_id, req_data, req_len, &resp_data, &resp_len) -> int` | 同步处理请求，返回 0 成功 |

- 请求/响应使用 protobuf 序列化（C/Zig/Nim 用 nanopb，Go 用 protobuf-go）
- 响应 buffer 由模块侧 `malloc` 分配，Rust 侧通过 `free()` 释放
- Go 模块额外支持双向流式通信（Send/Recv/CloseInput）

### Protobuf 协议（模块使用的核心消息）

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

## 添加新模块

1. 创建 `malefic-3rd-xxx/` crate，实现 `pub fn register(map: &mut MaleficBundle)`
2. 根 `Cargo.toml` 添加 feature + optional dependency
3. `src/lib.rs` 添加 `#[cfg(feature = "xxx_module")] malefic_3rd_xxx::register(&mut map);`

各语言的具体开发指南见对应子目录的 README。

## 构建与测试

```bash
# 构建（全部模块）
cargo build --target x86_64-pc-windows-gnu --features full --release

# 测试
cargo test --target x86_64-pc-windows-gnu --features full --release -- --nocapture

# 加载到 implant
load_module --path target/x86_64-pc-windows-gnu/release/malefic_3rd.dll
```
