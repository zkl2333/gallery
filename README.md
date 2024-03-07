# 海报墙生成器

这是一个用Rust编写的项目，主要功能是实时从指定目录中读取图像，并将这些图像处理、组合成一个大的海报墙。这个项目可以应用于生成图像集合预览、艺术作品展示墙等场景。

![poster](./poster.png)


## 如何使用

您可以通过以下步骤在本地机器上运行海报墙生成器。

```bash
gallery -d /path/to/your/images
```


## 开发环境
确保您的系统已经安装了Rust编程语言环境。如果您尚未安装Rust，请访问[Rust官网](https://www.rust-lang.org/learn/get-started)并按照指南完成安装。
如果您想在开发环境下运行项目，可以使用以下命令：

```bash
cargo run -- -d /path/to/your/images
```

这将启动程序，并使用`/path/to/your/images`目录下的图像文件生成海报墙。


## 如何贡献

欢迎对本项目做出贡献！如果您有任何改进建议或功能请求，可以通过以下方式参与：

1. Fork项目仓库。
2. 创建您的特性分支 (`git checkout -b feature/AmazingFeature`)。
3. 提交您的更改 (`git commit -m 'Add some AmazingFeature'`)。
4. 将您的更改推送到远程仓库 (`git push origin feature/AmazingFeature`)。
5. 创建一个Pull Request。


## 许可证

本项目采用MIT许可证。更多信息请查看`LICENSE`文件。
