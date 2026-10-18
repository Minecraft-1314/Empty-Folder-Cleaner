# Empty Folder Cleaner 空文件夹清理器

## 重要公告（必读）

- 安装程序尚未使用代码签名证书，Windows SmartScreen 可能显示安全提示。
- 本项目面向 Windows 10/11 x64。
- 所有版本均需要 Microsoft Edge WebView2 Runtime。
- 请先核对扫描结果，再执行删除操作。

## 项目介绍

这是一个面向 Windows 的本地空文件夹查找与清理工具。程序采用 **Rust 原生窗口外壳（tao + wry）+ WebView2 前端 + Rust 核心库 + 静态前端资源**的分层结构，支持并行扫描、结果筛选与排序、批量选择、级联清理、回收站删除、忽略规则、中英文切换以及明亮、暗色和跟随系统主题。

## 下载与安装

| 产物 | 用途 | 运行条件 |
|------|------|------|
| `EmptyFolderCleaner-Setup-0.0.1-win-x64.exe` | Inno Setup 标准安装向导，支持中文和英文 | Windows 10/11 x64，安装时需要管理员权限 |
| `EmptyFolderCleaner.msi` | 标准 Windows Installer 安装包 | Windows 10/11 x64，安装时需要管理员权限 |
| `EmptyFolderCleaner-portable-0.0.1-win-x64.zip` | 便携版压缩包，解压即用 | Windows 10/11 x64，需要 WebView2 Runtime |
| `dist/EmptyFolderCleaner-win-x64` | 免安装目录版 | Windows 10/11 x64，需要 WebView2 Runtime |

推荐普通用户使用 `EmptyFolderCleaner-Setup-0.0.1-win-x64.exe`。

## 项目贡献者

| 贡献者 | 贡献内容 |
|------|------|
| Minecraft-1314 | 完整开发 |

欢迎通过 Issues 或 PR 加入贡献者列表。

## 许可协议

本项目采用 MIT 许可证，详情参见 [LICENSE](LICENSE) 文件。

## 支持我们

如果这个项目对您有帮助，欢迎点亮 GitHub 仓库右上角的 Star。