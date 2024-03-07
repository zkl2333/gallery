use clap::{Arg, Command};
use image::{imageops, ImageBuffer, ImageError, Rgba};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), ImageError> {
    let matches = Command::new("海报墙生成器")
        .version("1.0")
        .author("多吃点 <i@zkl2333.com>")
        .about("实时从目录中的图像生成海报墙")
        .arg(
            Arg::new("directory")
                .short('d')
                .long("directory")
                .value_name("DIRECTORY")
                .help("指定图像所在的目录")
                .default_value("."),
        )
        .get_matches();

    let directory = matches
        .get_one::<String>("directory")
        .expect("directory 参数必须存在");

    // 创建针对终端的Layer
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_thread_ids(true);

    tracing_subscriber::registry().with(stdout_layer).init();

    info!("正在从 {} 目录下实时读取并处理图像", directory);

    create_poster_realtime(directory).await?;

    Ok(())
}

async fn create_poster_realtime(directory: &str) -> Result<(), ImageError> {
    const WIDTH: u32 = 4096;
    const HEIGHT: u32 = 2160;
    const IMG_WIDTH: u32 = 1000; // 每张图像调整后的宽度
    const IMG_HEIGHT: u32 = 400; // 每张图像调整后的高度
    const GAP: u32 = 10; // 图像间的间隙

    let poster: Arc<Mutex<ImageBuffer<Rgba<u8>, Vec<u8>>>> =
        Arc::new(Mutex::new(ImageBuffer::new(WIDTH, HEIGHT)));

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
                        debug!("正在压缩图像: {:?}", path.file_name().unwrap());
                        let img = img.resize(
                            IMG_WIDTH,
                            IMG_HEIGHT,
                            image::imageops::FilterType::Lanczos3,
                        );
                        let img = img.to_rgba8();
                        let (w, _h) = img.dimensions();

                        let mut poster = poster_clone.lock().await;
                        let mut x_guard = x_clone.lock().await;
                        let mut y_guard = y_clone.lock().await;
                        let mut line_guard = line_clone.lock().await;

                        if *y_guard > HEIGHT as i64 {
                            return;
                        }

                        debug!("正在绘制图像: {:?}", path.file_name().unwrap());
                        imageops::overlay(&mut *poster, &img, *x_guard, *y_guard);

                        if *x_guard < WIDTH as i64 {
                            *x_guard += w as i64 + GAP as i64;
                        } else {
                            *line_guard += 1;
                            *y_guard += IMG_HEIGHT as i64 + GAP as i64;
                            if *line_guard % 2 == 1 {
                                *x_guard = 0 - (IMG_HEIGHT as i64 / 2);
                            } else {
                                *x_guard = 0;
                            }
                        }

                        debug!("图像处理完成 坐标: ({}, {})", x_guard, y_guard);
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
