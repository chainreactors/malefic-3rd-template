# Malefic 3rd Party Module Template

这是一个用于创建 malefic implant 第三方模块的模板项目。该模板提供了开发和构建 malefic 模块的基础框架。

在Malefic本体中，选择了最小化依赖的设计模式，因此所有需要引入第三方模块的module，都将在3rd中实现。

在malefic 本体中也存在 https://github.com/chainreactors/malefic/tree/master/malefic-3rd ， 将提供一些community版本公开的3rd module。 

## 项目结构

```
malefic-3rd-template/
├── Cargo.toml           # 项目配置文件
├── src/
│   ├── lib.rs          # 主库文件，模块注册入口
│   ├── prelude.rs      # 公共导入
│   └── example/        # 示例模块
│       └── mod.rs      # 示例模块实现
└── README.md
```

## 使用方式

### 1. 构建模块

使用以下命令构建 DLL 文件：

```bash
cargo build -r
```

构建完成后，DLL 文件将位于 `target/release/` 目录中，文件名为 `malefic_3rd.dll`（Windows）或相应的动态库文件。

### 2. 加载模块

将构建生成的 DLL 文件加载到 malefic implant 中：

```bash
load_module --path <dll_path>
```

IoM client上执行
```bash
load_module --path target/release/malefic_3rd.dll
```

## Feature 配置

项目通过 Cargo features 来控制需要打包的模块，支持同时打包多个模块。

### 默认 Features

在 `Cargo.toml` 中配置：

```toml
[features]
default = ["as_cdylib", "example"]
as_cdylib = []
example = []
```

- `example`: 包含示例模块

### 自定义模块

要添加新的模块：

1. **创建模块目录和文件**：
   ```
   src/your_module/
   └── mod.rs
   ```

2. **在 Cargo.toml 中添加 feature**：
   ```toml
   [features]
   your_module = []
   ```

3. **在 src/lib.rs 中注册模块**：
   ```rust
   pub mod your_module;
   
   pub extern "C" fn register_3rd() -> MaleficBundle {
       let mut map: MaleficBundle = HashMap::new();
       
       #[cfg(feature = "example")]
       register_module!(map, "example", example::Example);
       
       #[cfg(feature = "your_module")]
       register_module!(map, "your_module", your_module::YourModule);
       
       map
   }
   ```

4. **实现模块结构**：
   ```rust
   use crate::prelude::*;
   
   pub struct YourModule {}
   
   #[async_trait]
   #[module_impl("your_module")]
   impl Module for YourModule {}
   
   #[async_trait]
   impl ModuleImpl for YourModule {
       async fn run(&mut self, id: u32, receiver: &mut crate::Input, sender: &mut crate::Output) -> ModuleResult {
           // 实现您的模块逻辑
           let request = check_request!(receiver, Body::Request)?;
           
           // 处理请求...
           
           let mut response = Response::default();
           response.output = "your module output".to_string();
           Ok(TaskResult::new_with_body(id, Body::Response(response)))
       }
   }
   ```

### 选择性构建

您可以通过指定 features 来选择性构建模块：

```bash
# 只构建 example 模块
cargo build -r --features "example"

# 构建多个模块
cargo build -r --features "example,your_module"

```

## 示例模块

项目包含一个示例模块 `example`，展示了如何：

- 使用 `#[module_impl]` 宏定义模块
- 实现 `Module` 和 `ModuleImpl` trait
- 处理请求和响应
- 返回执行结果

示例模块简单地返回一个字符串响应，可以作为开发新模块的参考。