use clap::{crate_authors, crate_description, crate_version, Arg, ArgAction, Command};
use image::{
    imageops, GenericImageView, ImageBuffer, ImageEncoder, ImageError, Pixel, Rgba, RgbaImage,
};
use imagequant::RGBA;
use oxipng::{optimize_from_memory, Options};
use std::{fs, sync::Arc};
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

// 压缩图像质量的函数
fn compress_png(input_img: &RgbaImage) -> RgbaImage {
    let mut liq = imagequant::new();
    liq.set_speed(4).unwrap();
    liq.set_quality(0, 70).unwrap();
    let mut rgba_vec: Vec<RGBA> = Vec::new();
    for pixel in input_img.pixels() {
        let rgba = pixel.to_rgba();
        rgba_vec.push(RGBA {
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3],
        });
    }
    let mut img = liq
        .new_image(
            rgba_vec,
            input_img.width().try_into().unwrap(),
            input_img.height().try_into().unwrap(),
            0.0,
        )
        .unwrap();

    // 进行量化处理
    let mut res = match liq.quantize(&mut img) {
        Ok(res) => res,
        Err(err) => panic!("量化处理失败: {err:?}"),
    };

    // 启用抖动以进行后续重新映射
    res.set_dithering_level(0.7).unwrap();

    // 可以重复使用结果生成具有相同调色板的多个图像
    let (palette, pixels) = res.remapped(&mut img).unwrap();

    info!(
        "完成！得到调色板 {} 和 {} 个像素，质量为 {}%",
        palette.len(),
        pixels.len(),
        res.quantization_quality().unwrap()
    );

    // 将调色板和像素重新映射到图像
    let mut img: RgbaImage = RgbaImage::new(input_img.width(), input_img.height());
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let index = pixels[(y * input_img.width() + x) as usize] as usize; // 获取当前像素对应的调色板索引
        let rgba = palette[index]; // 获取调色板中的颜色
        *pixel = Rgba([rgba.r, rgba.g, rgba.b, rgba.a]); // 设置像素颜色
    }

    let mut buf: Vec<u8> = Vec::new();
    image::codecs::png::PngEncoder::new(&mut buf)
        .write_image(&img, img.width(), img.height(), image::ColorType::Rgba8)
        .unwrap();

    // 设置oxipng的选项
    let options = Options::from_preset(2); // 使用预设级别2，你可以根据需要调整这个值

    // 使用optimize_from_memory函数优化PNG数据
    match optimize_from_memory(&buf, &options) {
        Ok(output_data) => {
            // 将优化后的数据写入文件
            fs::write("poster_compressed.png", &output_data).expect("Failed to write output file");
            println!("PNG optimization completed successfully.");
        }
        Err(e) => {
            eprintln!("Failed to optimize PNG: {:?}", e);
        }
    }

    img
}

async fn create_poster(
    directory: &str,
    width: u32,
    height: u32,
    gap: u32,
) -> Result<(), ImageError> {
    const IMG_WIDTH: u32 = 1000; // 每张图像调整后的宽度
    let img_height: u32 = (height - (gap * 9)) / 10; // 每张图像调整后的高度
    let img_height = if img_height < 100 { 100 } else { img_height };

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
                                image::imageops::FilterType::Lanczos3,
                                // image::imageops::FilterType::Nearest,
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
    poster
        .clone()
        .save_with_format("poster.png", image::ImageFormat::Png)?;

    info!("海报墙已保存为 poster.png");
    info!("正在压缩海报墙");
    let img = compress_png(&poster.clone());
    // img.save("poster_compressed.png")?;
    Ok(())
}
