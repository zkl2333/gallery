use clap::{crate_authors, crate_description, crate_version, Arg, ArgAction, Command};
use image::{imageops, ImageBuffer, ImageError, Rgba};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), ImageError> {
    let cmd = Command::new("海报墙生成器")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::new("help")
                .short('?')
                .long("help")
                .help("显示帮助信息")
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new("version")
                .short('v')
                .long("version")
                .help("显示版本信息")
                .action(ArgAction::Version),
        )
        .arg(
            Arg::new("directory")
                .short('d')
                .long("directory")
                .value_name("DIRECTORY")
                .help("指定图像所在的目录")
                .default_value("."),
        )
        .arg(
            Arg::new("width")
                .short('w')
                .long("width")
                .value_name("width")
                .help("指定海报墙的宽度")
                .default_value("4096"),
        )
        .arg(
            Arg::new("height")
                .short('h')
                .long("height")
                .value_name("height")
                .help("指定海报墙的高度")
                .default_value("2160"),
        )
        .arg(
            Arg::new("gap")
                .short('g')
                .long("gap")
                .value_name("gap")
                .help("指定图像之间的间隙")
                .default_value("10"),
        );

    let matches = cmd.get_matches();
    let directory: &String = matches.get_one::<String>("directory").unwrap();
    let width: u32 = matches.get_one::<String>("width").unwrap().parse().unwrap();
    let height: u32 = matches
        .get_one::<String>("height")
        .unwrap()
        .parse()
        .unwrap();
    let gap: u32 = matches.get_one::<String>("gap").unwrap().parse().unwrap();

    // 创建针对终端的Layer
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_thread_ids(true);

    tracing_subscriber::registry().with(stdout_layer).init();

    info!("正在从 {} 目录下实时读取并处理图像", directory);

    create_poster_realtime(&directory, width, height, gap).await?;

    Ok(())
}

async fn create_poster_realtime(
    directory: &str,
    width: u32,
    height: u32,
    gap: u32,
) -> Result<(), ImageError> {
    const IMG_WIDTH: u32 = 1000; // 每张图像调整后的宽度
    let img_height: u32 = (height - (gap * 5)) / 6; // 每张图像调整后的高度

    let poster: Arc<Mutex<ImageBuffer<Rgba<u8>, Vec<u8>>>> =
        Arc::new(Mutex::new(ImageBuffer::new(width, height)));

    let x: Arc<Mutex<i64>> = Arc::new(Mutex::new(0));
    let y: Arc<Mutex<i64>> = Arc::new(Mutex::new(0));
    let line: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    let mut entries = tokio::fs::read_dir(directory).await?;

    // 为了并行处理图像，收集所有的异步任务
    let mut tasks = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let poster_clone = poster.clone();
        let x_clone = x.clone();
        let y_clone = y.clone();
        let line_clone = line.clone();
        if path.is_file() {
            let task = tokio::spawn(async move {
                match image::open(&path) {
                    Ok(img) => {
                        let mut y_guard = y_clone.lock().await;
                        if *y_guard > height as i64 {
                            return;
                        }
                        debug!("正在压缩图像: {:?}", path.file_name().unwrap());
                        let img = img.resize(
                            IMG_WIDTH,
                            img_height,
                            image::imageops::FilterType::Lanczos3,
                        );
                        let img = img.to_rgba8();
                        let (w, _h) = img.dimensions();

                        let mut poster = poster_clone.lock().await;
                        let mut x_guard = x_clone.lock().await;
                        let mut line_guard = line_clone.lock().await;

                        if *y_guard > height as i64 {
                            return;
                        }

                        debug!("正在绘制图像: {:?}", path.file_name().unwrap());
                        imageops::overlay(&mut *poster, &img, *x_guard, *y_guard);
                        debug!("图像处理完成 坐标: ({}, {})", x_guard, y_guard);

                        if *x_guard < width as i64 {
                            *x_guard += w as i64 + gap as i64;
                        } else {
                            *line_guard += 1;
                            *y_guard += img_height as i64 + gap as i64;
                            if *line_guard % 2 == 1 {
                                *x_guard = 0 - (img_height as i64 / 2);
                            } else {
                                *x_guard = 0;
                            }
                        }
                    }
                    Err(e) => error!("无法打开图像 {:?}: {:?}", path, e),
                }
            });
            tasks.push(task);
        }
    }

    // 等待所有任务完成
    for task in tasks {
        let _ = task.await;
    }

    poster.lock().await.save("poster.png")?;

    info!("海报墙已保存为 poster.png");

    Ok(())
}
