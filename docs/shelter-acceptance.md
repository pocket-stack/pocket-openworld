# 用窗口验收交互

```sh
cargo run --locked -- --shelter
```

启动后直接显示 XP 风格的控制窗口。普通果园里按 **Tab** 打开同一个窗口。
窗口打开时鼠标可见，点击不会转动镜头或移动角色；可以拖动蓝色标题栏。
点右上角 **X**、**Back to game** 或再按 Tab，恢复游戏控制。

## 最快的验收方式

点选分页，再点主按钮即可。无需记忆操作键、走位或掐时间。
自动流程使用实际点火、喷水和移动操作；结果由物理化学状态判断。
完成后显示 **PASS / FAIL** 并暂停，方便观察。**Reset** 可以重复试验。

| 分页 | 主按钮 | 应看到的结果 | 第二个按钮 |
| --- | --- | --- | --- |
| Rain | Run rain & roof comparison，15 秒 | 棚下木柴先着火；屋顶移走后，雨水将火浇灭 | Keep roof in place：8 秒后棚下着火、露天不燃 |
| Screen | Run heat comparison，20 秒 | 挡板后的木柴仍湿；露出的木柴烘干并着火 | Open screen & spray：约 42 秒内先让两侧烘干着火，再喷水熄灭 |
| Firebreak | Run wet firebreak，30 秒 | 左排烧穿；右排中间的湿草阻止蔓延，末端仍绿 | Run without water：两排都会烧穿 |

底部的 Live observations 显示左右湿度、温度和燃烧状态。
防火带同时显示火是否曾经到达末端，避免错过火苗后无法判断。

## 自由点击组合

**Try it yourself** 中可以直接点：

- **Ignite / Light heaters**：给试验起点点火。
- **Spray water**：使用可见的喷头，向木柴或中间草带喷水。
- **Start / Stop rain**：切换降雨。
- **Roof to left / right、Open / Close screen**：移动遮挡物。

手动操作会停止当前自动流程并继续模拟。不可用的控件会变灰。
**Pause / Resume** 用于停下观察、继续反应；**Reset** 恢复当前试验、天气和所有记录。
关闭控制窗口会停止自动流程并恢复自由游戏，模拟不会留在暂停状态。

**Orchard** 分页返回原版芙莉莲所在的果园，提供法杖挥击、点火、喷水和拾取按钮。
关闭窗口后仍可用 WASD 移动、鼠标瞄准；需要动作或实验入口时再按 Tab。

## HUD 字体

关闭控制窗口后，果园标题、喷水状态、目标读数、底部通知，以及实验面板和
场景中的 Left / Right 标签都使用与 XP 窗口相同的 Inter 字体。
字体和面板随显示缩放，通知按真实字宽自动换行。HUD 不接管鼠标或键盘。
可先在 Orchard 点 Spray water，再关闭窗口观察喷水状态和底部通知；
在 Rain 点主按钮运行完成后关闭窗口，可查看实验读数与场景标签。

## 自动复核

```sh
cargo test --locked
cargo build --locked
python3 tools/verify_shelter.py --output target/acceptance/controls
```

脚本从待测二进制读取嵌入的 UI 构建清单，核对源码、编译器和字体哈希，
拒绝使用旧 UI 的二进制，再运行 26 个场景：8 个窗口点击场景、
8 个化学对照、10 个原有场景。点击测试从 PocketJS 原生核心的实际控件位置
注入鼠标按下和释放，经过同一套焦点与 onPress 处理；再比较完整状态回放并输出 PNG。

单独查看窗口对照：

```sh
cargo run --locked -- --headless --scenario controls-firebreak --ticks 1806 \
  --receipt /tmp/firebreak-ui.json --screenshot /tmp/firebreak-ui.png
```

2x 显示缩放：加 `--size 1920x1200 --ui-scale 2`。
状态回执证明模拟和输入结果，截图证明窗口及世界的显示。

构建前安装 `.bun-version` 指定的 Bun 和 `rust-toolchain.toml` 指定的 Rust。
修改 UI 后照常运行：

```sh
cargo run --locked
```

Cargo 自动安装锁定的 JS 依赖、生成 JS／字体 PAK 并嵌入二进制，产物只写到
`target` 下。构建后的游戏不需要 Bun。完整验收输出放在本地 `target/acceptance`
或 CI 的 `pocket-openworld-acceptance` artifact 中，保留 30 天；它们不提交到 Git。
PR 验收请核对 workflow 对应的提交，再下载其 artifact 查看 PNG、JSON 和日志。
