use clap::{Arg, Command};
use image::{imageops, ImageBuffer, ImageError, Rgba};
use std::fs;

fn main() -> Result<(), ImageError> {
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

    println!("正在从 {} 目录下实时读取并处理图像", directory);

    let poster = create_poster_realtime(directory)?;

    poster.save("poster.png")?;

    println!("海报墙已保存为 poster.png");
    Ok(())
}

fn create_poster_realtime(directory: &str) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, ImageError> {
    const WIDTH: u32 = 1000;
    const HEIGHT: u32 = 1000;
    const IMG_WIDTH: u32 = 1000; // 每张图像调整后的宽度
    const IMG_HEIGHT: u32 = 120; // 每张图像调整后的高度
    const GAP: u32 = 10; // 图像间的间隙

    let mut poster = ImageBuffer::new(WIDTH, HEIGHT);
    let mut x: i64 = 0;
    let mut y: i64 = 0;
    let mut line: u32 = 0;

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            match image::open(&path) {
                Ok(img) => {
                    if y > HEIGHT.try_into().unwrap() {
                        break;
                    }
                    println!("正在处理图像 {:?}", path);
                    let img =
                        img.resize(IMG_WIDTH, IMG_HEIGHT, image::imageops::FilterType::Nearest);
                    let img = img.to_rgba8();
                    let (w, _h) = img.dimensions();
                    imageops::overlay(&mut poster, &img, x, y);

                    if x < WIDTH as i64 {
                        x += w as i64 + GAP as i64;
                    } else {
                        line += 1;
                        y += IMG_HEIGHT as i64 + GAP as i64;
                        if line % 2 == 1 {
                            x = 0 - (IMG_HEIGHT as i64 / 2);
                        } else {
                            x = 0;
                        }
                    }
                }
                Err(e) => println!("无法打开图像 {:?}: {:?}", path, e),
            }
        }
    }

    Ok(poster)
}
