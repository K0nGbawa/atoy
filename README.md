**Atoy（一个玩具）** 是由 K0nGbawa 和 Zes Minkey Young 制作的解释型编程语言项目，使用 Rust 开发解释器，采用「词法分析 → 语法分析（编译为字节码）→ 栈式虚拟机」的架构，并附带一个多行 REPL。

## 快速开始

需要 [Rust](https://www.rust-lang.org/) 工具链（`cargo`）。

```bash
# 构建
cargo build --release

# 进入 REPL
cargo run

# 运行脚本文件
cargo run -- example_code/fib.at
```

- 无参数运行进入 REPL，输入 `exit()` 退出。
- 带文件路径参数时直接执行该脚本；REPL 支持跨行输入（语句不完整时会继续等待下一行）。
- 所有示例脚本位于 [`example_code/`](example_code/)：

| 文件 | 内容 |
| --- | --- |
| `hello.at` | 打印字符串与转义序列 |
| `fib.at` | 递归闭包 + `while` 循环输出斐波那契数列 |
| `table.at` | 表、元表、原型与运算符重载（`+`） |
| `closure.at` | 闭包捕获外层环境（计数器工厂） |

## 目录结构

```
atoy/
├── Cargo.toml            # workspace 清单
├── crates/
│   ├── atoy/             # 解释器主体
│   │   └── src/
│   │       ├── main.rs       # 入口：REPL / 执行脚本文件
│   │       ├── lib.rs        # 库导出
│   │       ├── lexer.rs      # 词法分析器
│   │       ├── parser.rs     # 语法分析器 + 编译为字节码（OpCode）
│   │       ├── vm.rs         # 栈式虚拟机
│   │       └── builtin.rs    # 内置函数与方法的实现
│   └── atoy-macros/     # 过程宏 crate：内置函数的注册声明
└── example_code/         # .at 示例脚本
```

## 语法

### 关键字

- `let` 声明变量
- `fn` 声明匿名函数（闭包）
- `if` `else` 条件语句
- `while` 循环语句
- `return` 返回值
- `and` `or` `not` 逻辑运算符
- `true` `false` 布尔字面量

### 字面量与注释

- 整数：`42`；浮点数：`3.14`
- 字符串：单引号或双引号均可，支持转义 `\n` `\t` `\r` `\"` `\'` `\\`，以及 `\uXXXX` Unicode 转义；字符串内不允许出现裸换行
- 表：`{}`（字段以键值形式动态赋值）
- 数组：`[]`
- 行注释：`//` 注释到行尾

### 运算符

- 算术运算符：`+` `-` `*` `/`
- 比较运算符：`==` `!=` `<` `>` `<=` `>=`
- 级联运算符：`..`（Lua 风格，自动隐式转换拼接）
- 赋值：`=` 与复合赋值 `+=` `-=` `*=` `/=`
- 逻辑运算符：`and` `or` `not`
- 成员/方法/索引：
  - `obj.field` 属性访问（语法糖）
  - `obj:method(args)` 方法调用：自动把 `obj` 作为第一个参数传入（类似 Lua 的 `:`）
  - `obj[key]` 索引访问
- 函数调用：`fn_name(args)`

### 表达式

优先级从低到高：

- 函数定义（`fn`）
- 级联运算（类 Lua，支持隐式转换）
- 或运算
- 与运算
- 比较运算
- 算术运算
- 函数调用 / 索引访问 / 成员访问（为语法糖，类似 JavaScript 和 Lua）
- 字面量

### 语句示例

```js
// let + 闭包
let fib = fn(n) {
    if n == 1 or n == 2 return 1;
    return fib(n - 1) + fib(n - 2);
};

// while + 复合赋值
let n = 1;
while n < 20 {
    println(fib(n));
    n = n + 1;
}
```

## 数据类型及内部 Rust 类型

- 整数（`i64`）
- 浮点数（`f64`）
- 布尔值（`bool`）
- 字符串（`Rc<String>`）
- 数组（`Rc<RefCell<Vec<Value>>>`）
- 表（`Rc<RefCell<Table>>`），Table 含有 `HashMap<Value, Value>`
- 集合（`Rc<RefCell<HashSet<Value>>>`，语法待实现）
- 内置函数（`Rc<dyn Fn(Args) -> RuntimeResult<Value>>`）与用户函数（`Rc<Func>`，保存字节码与捕获的环境）
- `None`（空值）

真值规则：`None` 为假；整数/浮点 `0` 为假；布尔值取自身；函数恒为真；其余类型的真值语义按各类型定义。

## 元表与原型

类似 Lua 和 JavaScript，Atoy 的表支持元表（metatable）与原型（prototype），两者分离：**元表负责运算符重载，原型负责查找继承**。

- 原型：访问表中不存在的字段时，会沿原型链继续查找（`getPrototypeOf` / `setPrototypeOf` / `clearPrototypeOf` 管理）。
- 元表：可在元表中定义 `add` 等键，为表重载运算符（`getMetatableOf` / `setMetatableOf` / `clearMetatableOf` 管理）。

```js
let meta = {};
let Class = {};

Class.new = fn() {
    let a = {};
    setMetatableOf(a, meta);   // 挂元表：支持运算符重载
    setPrototypeOf(a, Class);  // 挂原型：支持字段继承
    return a;
};

// 运算符重载：a + b 时调用 meta.add(a, b)
meta.add = fn(self, other) {
    return self:add(other);    // ':' 把 self 作为首参传给 Class.add
};

Class.add = fn(self, other) {
    return [self, other];
};

let obj = Class.new();
println(obj + obj);            // 输出两个对象的数组
```

## 内置函数

### 全局函数

- `println(...)` 打印参数，参数之间用空格分隔
- `input(prompt)` 打印提示（可选），读取一行输入
- `repr(value)` 递归格式化任意值（循环引用会输出引用编号）
- `type(value)` 返回值的类型名字符串
- `Table()` 新建一个空表
- 原型管理：`getPrototypeOf(target)` / `setPrototypeOf(target, proto)` / `clearPrototypeOf(target)`
- 元表管理：`getMetatableOf(target)` / `setMetatableOf(target, meta)` / `clearMetatableOf(target)`

### 字符串方法（`String` 表 / 字符串索引调用）

注册在字符串原型上，可用 `"..." : 方法名(...)` 调用：

- `len()` 返回字节长度
- `toInteger(base)` 按进制解析为整数（base 可选，默认 10）
- `lower()` / `upper()` 转小写 / 大写
- `from(value)` 将其他值转换为字符串（`String` 表的转换入口）

示例：`"Hello":len()`、`"ff":toInteger(16)`、`"abc":upper()`。

### 数组方法（`Array` 表 / 数组索引调用）

- `new()` 新建空数组（等价于 `[]`）
- `len()` 返回元素个数
- `push(value)` 尾部追加
- `pop()` 弹出尾部元素（空数组返回 `None`）

示例：`Array:new()`、`[1, 2]:push(3)`。

## 扩展内置函数（atoy-macros）

内置函数通过 `crates/atoy-macros` 提供的过程宏声明，自动生成参数校验、类型转换（`TryFrom<&Value>`）与错误包装，无需手写胶水代码：

```rust
use atoy_macros::atoy_function;

#[atoy_function]
pub fn println(args: Args) { /* ... */ }

#[atoy_function(method = len)]
pub fn String_len(val: String) -> i64 {
    val.len() as i64
}
```

- `#[atoy_function]`：注册为**全局函数**（函数名即脚本中调用名）。
- `#[atoy_function(method = name)]`：注册为**原型上的方法**，方法名为 `name`，函数名约定为 `类型_方法`（如 `String_len` 属于字符串方法，但命名不强制要求）。
- 函数签名支持：
  - 普通参数按 `TryFrom<&Value>` 自动转换（`i64`、`f64`、`String`、`bool`、`&Value`、数组/表的 `Rc` 引用等）；
  - `Option<T>` 表示可选参数（必须在必选参数之后）；
  - 末尾的 `Args` 直接接收全部原始参数；
  - 返回类型省略表示返回 `None`，否则自动包装为 `Value`。
- 注册时通过 `vm.register_func(...)`（全局）或原型表的 `register_methods!`（方法）把宏生成的 `__atoy_register_*` 函数登记进虚拟机（见 `vm.rs` 的 `VM::new` 与 `builtin.rs`）。
