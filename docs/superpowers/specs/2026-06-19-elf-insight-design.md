# ELF Insight — TUI ELF 查看器设计文档

**日期**: 2026-06-19
**状态**: 设计完成

---

## 1. 概述

一个基于 Rust + Ratatui 的 TUI ELF 文件查看解析工具。支持浏览 ELF 文件布局、查看 Section/Segment 的十六进制和字符信息、对代码段进行反汇编并展示函数级结构。

## 2. 技术选型

| 组件 | 选择 | 说明 |
|------|------|------|
| 语言 | Rust | 用户指定 |
| TUI 框架 | Ratatui | 成熟稳定，生态良好 |
| ELF 解析 | `goblin` | 纯 Rust，支持 ELF/PE/Mach-O |
| 反汇编 | `iced-x86` | 纯 Rust，x86/x64，零编译依赖 |
| 搜索 | 自实现 | 基于线性扫描 + 字符串匹配 |

## 3. 功能范围

### 3.1 核心功能

- 命令行传入 ELF 文件路径，单文件查看
- 启动后默认展示 Overview 页面，类似 `readelf -WSlh` 的汇总信息（ELF Header + Section 表 + Segment 表），可滚动浏览
- 左侧折叠树导航：Overview / ELF Header / Sections / Segments / Symbols
- 右侧详情面板，根据选中节点类型自动切换视图：
  - 结构化信息（字段表格）
  - Hexdump（十六进制 + ASCII，可编辑光标）
  - 反汇编（函数列表 + 指令级反汇编）
  - 字符串视图
- 搜索：`/` 弹出搜索栏，支持符号名、地址、字符串搜索，`n`/`N` 跳转

### 3.2 不在范围

- 文件修改/编辑
- 多文件同时查看
- 导出功能
- 非 x86/x64 架构
- 文件浏览器

## 4. 架构

```
┌──────────────────────────────────────────────────────┐
│                    CLI Args (file path)               │
├──────────────────────────────────────────────────────┤
│                    App (Ratatui)                      │
│  ┌──────────────┐  ┌────────────────────────────────┐│
│  │  Left Panel  │  │        Right Panel              ││
│  │  (Tree Nav)  │  │  ┌────────────────────────────┐││
│  │              │  │  │ Detail View                │││
│  │  Overview     │  │  │  • Overview (readelf-like) │││
│  │  ├─Sections  │  │  │  • Hexdump                 │││
│  │  ├─Segments  │  │  │  • Disassembly             │││
│  │  └─Symbols   │  │  │  • Strings                 │││
│  │              │  │  └────────────────────────────┘││
│  │              │  │  ┌────────────────────────────┐││
│  │              │  │  │ Search Bar (/ to open)     │││
│  └──────────────┘  │  └────────────────────────────┘││
│                     └────────────────────────────────┘│
├──────────────────────────────────────────────────────┤
│  Core: ELF Parser (goblin) + Disasm (iced-x86)       │
└──────────────────────────────────────────────────────┘
```

### 4.1 模块划分

| 模块 | 职责 | 依赖 |
|------|------|------|
| `elf/parser` | 封装 goblin，解析 ELF，组织为内部数据结构 | goblin |
| `elf/disasm` | 封装 iced-x86，将字节码反汇编为指令列表，识别函数边界 | iced-x86 |
| `ui` | Ratatui 渲染、事件处理、状态管理 | ratatui, crossterm |
| `ui/widgets` | 各视图组件（tree、hexdump、disasm、info、search） | ratatui |
| `app` | 顶层 Application，协调各模块 | 以上所有 |

### 4.2 数据流

```
File → goblin::Object → ElfData (内部结构)
                           ↓
              App 持有 ElfData + UI State
              ↓              ↓
         Left Panel      Right Panel
         (Tree State)    (Detail View State)
```

## 5. 左侧面板 — 导航树

### 5.1 树结构

```
Overview
ELF Header
├─ Sections
│  ├─ .interp
│  ├─ .note.*
│  ├─ .hash / .gnu.hash
│  ├─ .dynsym
│  ├─ .dynstr
│  ├─ .text
│  ├─ .rodata
│  ├─ .data
│  ├─ .bss
│  └─ ...
├─ Segments
│  ├─ PHDR
│  ├─ LOAD [0]
│  ├─ LOAD [1]
│  ├─ DYNAMIC
│  └─ ...
└─ Symbols
   ├─ [F] main
   ├─ [F] printf@plt
   ├─ [O] stdout
   └─ ...
```

### 5.2 交互

- `↑↓` 移动焦点
- `→` / `Enter` 展开节点
- `←` 折叠节点
- 初始状态：ELF Header 和顶层分组（Sections、Segments、Symbols）默认展开，子节点折叠
- 选中节点后右侧面板自动切换对应视图
- 节点类型 → 视图类型映射：
  - Overview → 全景汇总视图（ELF Header + Section 表 + Segment 表）
  - ELF Header → 结构化字段表格
  - Section Header → 结构化字段表格 + hexdump 预览
  - Section Body → hexdump / 反汇编（根据 section 类型）
  - Program Header → 结构化字段表格
  - Symbol → 反汇编（函数）或地址跳转（对象）

## 6. 右侧面板 — 详情视图

### 6.1 全景汇总视图（Overview）

启动时默认展示，类似 `readelf -WSlh` 的输出。包含 ELF Header 关键字段、Section 表、Segment 表三部分，可滚动浏览。

```
┌─ Overview ─ /bin/ls ───────────────────────────────────┐
│  ELF Header                                             │
│    Magic:  7f 45 4c 46 02 01 01 00 00 00 00 00 00 ...  │
│    Class:  ELF64                                        │
│    Type:   DYN (Shared object file)                     │
│    Machine: x86-64                                      │
│    Entry:  0x6bb0                                       │
│                                                         │
│  Section Headers:  [Nr] Name         Type      Address  │
│    [ 0]                   NULL       0         0        │
│    [ 1] .interp            PROGBITS   0x318     0x31c   │
│    [ 2] .note.gnu.property NOTE       0x338     0x340   │
│    ...                                                  │
│                                                         │
│  Program Headers:   Type  Offset   VirtAddr  PhysAddr   │
│    PHDR             0x40   0x40     0x40     ...        │
│    INTERP           0x318  0x318    0x318    ...        │
│    LOAD             0x0    0x0      0x0      ...        │
│    ...                                                  │
└─────────────────────────────────────────────────────────┘
```

交互：`↑↓` / `PgUp/PgDn` 滚动，只读展示。

### 6.2 结构化信息视图（单字段详情）

表格形式展示结构化字段，用于 ELF Header、Section Header、Program Header、Symbol 等。

```
┌─ ELF Header ─────────────────────────────────────────┐
│  Magic:        7f 45 4c 46 02 01 01 00 00 00...      │
│  Class:        ELF64                                  │
│  Data:         2's complement, little endian          │
│  Version:      1 (current)                            │
│  OS/ABI:       UNIX - System V                        │
│  Type:         DYN (Shared object file)               │
│  Machine:      Advanced Micro Devices X86-64          │
│  Entry:        0x10a0                                 │
│  ...                                                  │
└───────────────────────────────────────────────────────┘
```

### 6.3 Hexdump 视图

```
┌─ .rodata ─ 0x2000-0x2156 ─────────────────────────────┐
│  Offset   │ 00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F │ ASCII     │
│  ────────────────────────────────────────────────────────────── │
│  0x0002000│ 01 00 02 00 00 00 00 00  00 00 00 00 00 00 00 00 │ ················│
│  0x0002010│ 48 65 6c 6c 6f 20 57 6f  72 6c 64 21 00 00 00 00 │ Hello World!···│
│  ...                                                           │
└─────────────────────────────────────────────────────────────────┘
```

交互：
- `↑↓` 逐行 / 逐字节滚动
- `PgUp/PgDn` 翻页
- `g` 弹出输入框，跳转到指定偏移
- `Tab` 在 hex 区和 ASCII 区之间切换光标
- 选中字节高亮，hex 和 ASCII 区联动高亮
- 不可打印字符显示为 `·`

### 6.4 反汇编视图

```
┌─ .text ─ main ─ 0x10a0-0x11f8 ────────────────────────┐
│  Functions:  main | foo | bar | ...                    │
│  ─────────────────────────────────────────────────────  │
│  0x10a0:  55                  push    rbp              │
│  0x10a1:  48 89 e5            mov     rbp, rsp         │
│  0x10a4:  48 83 ec 10         sub     rsp, 0x10        │
│  0x10a8:  48 8d 05 51 0f 00   lea     rax, [rip+0xf51]│
│  ...                                                    │
│  ; 0x10f0:  jmp 0x10a0                                  │
└─────────────────────────────────────────────────────────┘
```

交互：
- 上方函数列表：`←→` 切换函数
- `↑↓` 逐指令滚动
- 指令格式：`地址: 机器码(hex) 助记符 操作数`
- 跳转目标标注（箭头或注释）
- 当前指令高亮

函数识别策略：
- 优先从符号表中读取函数符号及其地址
- 对于 strip 后的二进制，从 entry point 开始线性扫描，通过 `call`/`jmp` 和 `ret` 指令识别函数边界

### 6.5 字符串视图

当选中 `.dynstr`、`.strtab` 等字符串表 section 时显示。以列表形式列出所有以 null 结尾的可打印字符串。

```
┌─ .dynstr ─ 0x500-0x6a8 ───────────────────────────────┐
│  0x500  /lib64/ld-linux-x86-64.so.2                    │
│  0x51c  libc.so.6                                      │
│  0x525  printf                                         │
│  0x52c  malloc                                         │
│  0x533  __libc_start_main                              │
│  ...                                                    │
└─────────────────────────────────────────────────────────┘
```

交互：
- `↑↓` 滚动选择
- 选中字符串高亮
- 搜索 `/` 时自动匹配字符串内容

### 6.6 搜索

- `/` 弹出搜索输入栏
- 输入关键词后回车执行搜索
- 搜索类型自动判断：
  - `0x` 开头 → 地址搜索
  - 可打印字符串 → 符号名搜索（优先）或字符串内容搜索
- `n` 跳转到下一个匹配
- `N` 跳转到上一个匹配
- 搜索结果高亮，匹配时自动滚动到目标位置
- 搜索范围限定在当前视图（在 hexdump 中搜 hex 内容，在 disasm 中搜指令，在 tree 中搜符号名）
- 无匹配结果时在搜索栏显示 "No matches"，3 秒后自动消失
- 搜索输入过程中的实时过滤（仅 tree 视图，在符号名中实时过滤）

## 7. 快捷键总览

| 键 | 上下文 | 功能 |
|----|--------|------|
| `q` | 全局 | 退出 |
| `/` | 全局 | 打开搜索栏 |
| `Esc` | 搜索栏 | 关闭搜索栏 |
| `n` / `N` | 搜索 | 下一个/上一个匹配 |
| `↑↓` | 树/列表 | 移动焦点 |
| `→` / `Enter` | 树 | 展开节点 |
| `←` | 树 | 折叠节点 |
| `Tab` | Hexdump | 切换 hex/ASCII 光标 |
| `PgUp/PgDn` | Hexdump/Disasm | 翻页 |
| `g` | Hexdump | 跳转到指定偏移 |
| `←→` | Disasm | 切换函数 |

## 8. 依赖 (Cargo.toml)

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
goblin = "0.9"
iced-x86 = "1.21"
```

## 9. 文件结构

```
elf-insight/
├── Cargo.toml
├── src/
│   ├── main.rs          # 入口，CLI 参数解析
│   ├── app.rs           # Application 状态机
│   ├── elf/
│   │   ├── mod.rs
│   │   ├── parser.rs    # goblin 封装，内部数据结构
│   │   └── disasm.rs    # iced-x86 封装
│   └── ui/
│       ├── mod.rs       # 主布局渲染
│       ├── tree.rs      # 左侧导航树
│       ├── info.rs      # 结构化信息视图
│       ├── hexdump.rs   # Hexdump 视图
│       ├── disasm.rs    # 反汇编视图
│       └── search.rs    # 搜索栏
└── docs/
    └── superpowers/
        └── specs/
            └── 2026-06-19-elf-insight-design.md
```

## 10. 测试策略

- ELF 解析：使用已知的 ELF 测试文件（如 `/bin/ls`），验证解析结果与 `readelf` 一致
- 反汇编：验证 iced-x86 输出与 `objdump -d` 一致（仅对比几条已知指令）
- UI：Ratatui 不便于单元测试，依赖手动测试。核心逻辑（解析、搜索）与 UI 解耦，可独立测试