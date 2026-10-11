# Empty Folder Cleaner 空文件夹清理器

## 重要公告（必读）

- 安装程序尚未使用代码签名证书，Windows SmartScreen 可能显示安全提示。
- 所有版本均需要 Microsoft Edge WebView2 Runtime。
- 程序仅围绕文件夹扫描与删除工作，不需要账号、站点凭据或云端数据。
- 请先核对扫描结果，再执行删除操作。

## 项目介绍

这是一个面向 Windows 的本地空文件夹查找与清理工具，当前版本为 `0.0.1`。程序采用 **WPF 外壳 + WebView2 前端 + C# 核心库 + 静态前端资源**的分层结构，支持并行扫描、结果筛选与排序、批量选择、级联清理、回收站删除、忽略规则、中英文切换以及明亮、暗色和跟随系统主题。

前端资源全部本地化，不依赖运行时 CDN；首次启动且未保存设置前，不会创建配置文件。

## 下载与安装

| 产物 | 用途 | 运行条件 |
|------|------|------|
| `EmptyFolderCleaner-Setup-0.0.1-win-x64.exe` | Inno Setup 7.1.0 标准安装向导，支持中文和英文 | Windows 10/11 x64，安装时需要管理员权限 |
| `EmptyFolderCleaner-0.0.1-win-x64.msi` | 标准 Windows Installer 安装包 | Windows 10/11 x64，安装时需要管理员权限 |
| `EmptyFolderCleaner-Default-0.0.1-win-x64.zip` | 默认版压缩包 | 已内置 .NET 运行时，需要 WebView2 Runtime |
| `EmptyFolderCleaner-Portable-0.0.1-win-x64.zip` | 单文件便携版压缩包 | 已内置 .NET 运行时，需要 WebView2 Runtime |

推荐普通用户使用 `EmptyFolderCleaner-Setup-0.0.1-win-x64.exe`。已安装版本可通过 Windows 标准应用管理功能卸载，`dist` 目录中的 `uninstall.exe` 会调用对应的 Inno Setup 或 Windows Installer 卸载流程；便携版只需删除解压目录。

## 快速开始

1. 选择需要扫描的目录。
2. 启动扫描并等待结果加载完成。
3. 使用筛选、排序、全选或取消全选整理结果。
4. 确认删除方式后执行清理；新变空的父目录会继续参与级联清理。
5. 可在忽略规则中添加通配符，或在设置中清空已保存配置。

## 主要功能

| 功能 | 说明 |
|------|------|
| 并行扫描 | 按目录并行遍历，结果按批次显示 |
| 级联清理 | 子目录删除后，新变空的父目录继续清理 |
| 安全删除 | 支持 Windows 回收站和永久删除 |
| 结果管理 | 支持筛选、路径排序、修改时间排序、批量选择和空间估算 |
| 忽略规则 | 支持通配符规则和显式忽略路径 |
| 双语界面 | 内置简体中文和英文 |
| 响应式界面 | 适配桌面、平板和手机尺寸 |

## 许可协议

本项目采用 MIT 许可证，详情参见 [LICENSE](LICENSE) 文件。
