use clap::{crate_authors, crate_description, crate_version, Arg, ArgAction, Command};
use image::{imageops, GenericImageView, ImageBuffer, ImageError, Rgba};
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

    create_poster(&directory, width, height, gap).await?;

    Ok(())
}

// 获取图片的宽度和高度
fn get_image_dimensions(img: &image::DynamicImage) -> (u32, u32) {
    let (width, height) = img.dimensions();
    (width, height)
}

// 计算 resize 后的宽度和高度 (保持原始宽高比, 但高度固定)
fn calculate_new_dimensions(
    img: &image::DynamicImage,
    _new_width: u32,
    new_height: u32,
) -> (u32, u32) {
    let (width, height) = get_image_dimensions(img);
    let ratio = height as f32 / new_height as f32;
    let new_width = (width as f32 / ratio) as u32;
    (new_width, new_height)
}

async fn create_poster(
    directory: &str,
    width: u32,
    height: u32,
    gap: u32,
) -> Result<(), ImageError> {
    const IMG_WIDTH: u32 = 1000; // 每张图像调整后的宽度
    let img_height: u32 = (height - (gap * 9)) / 10; // 每张图像调整后的高度

    let mut x: i64 = 0;
    let mut y: i64 = 0;
    let mut line: u32 = 0;

    let mut entries = tokio::fs::read_dir(directory).await?;

    let poster: Arc<Mutex<ImageBuffer<Rgba<u8>, Vec<u8>>>> =
        Arc::new(Mutex::new(ImageBuffer::new(width, height)));

    // 为了并行处理图像，收集所有的异步任务
    let mut tasks = Vec::new();

    // 首尾相连的循环
    'outer: loop {
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                match image::open(&path) {
                    Ok(img) => {
                        if y > height as i64 {
                            break 'outer;
                        }

                        // 计算 resize 后的宽度和高度
                        let (w, _h) = calculate_new_dimensions(&img, IMG_WIDTH, img_height);

                        // 分配位置
                        let task_x = x;
                        let task_y = y;

                        // 更新坐标
                        if x < width as i64 {
                            x += w as i64 + gap as i64;
                        } else {
                            line += 1;
                            y += img_height as i64 + gap as i64;
                            if line % 2 == 1 {
                                x = 0 - (img_height as i64 / 2);
                            } else {
                                x = 0;
                            }
                        }

                        debug!("分配位置: ({}, {})", x, y);

                        let poster = poster.clone();
                        // 异步处理图像
                        let task = tokio::spawn(async move {
                            debug!("正在压缩图像: {:?}", path.file_name().unwrap());
                            let img = img.resize(
                                IMG_WIDTH,
                                img_height,
                                // image::imageops::FilterType::Lanczos3,
                                image::imageops::FilterType::Nearest,
                            );
                            let mut poster = poster.lock().await;
                            imageops::overlay(&mut *poster, &img, task_x, task_y);
                            debug!(
                                "图像 {:?} 处理完成 坐标: ({}, {})",
                                path.file_name().unwrap(),
                                task_x,
                                task_y
                            );
                        });
                        tasks.push(task);
                    }
                    Err(e) => error!("无法打开图像 {:?}: {:?}", path, e),
                }
            }
        }
        entries = tokio::fs::read_dir(directory).await?;
    }

    // 等待所有任务完成
    for task in tasks {
        let _ = task.await;
    }

    // 保存海报墙
    info!("正在保存海报墙");
    let poster = poster.lock().await;
    poster.save_with_format("poster.png", image::ImageFormat::Png)?;

    Ok(())
}
