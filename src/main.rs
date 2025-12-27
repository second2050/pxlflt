use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use image::{DynamicImage, GenericImageView, ImageReader, Rgba};
use rand::seq::SliceRandom;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::Mutex,
};
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, disable_help_flag = true)]
struct Args {
    #[arg(short, long)]
    server: String,

    #[arg(short, long)]
    image: String,

    #[arg(short, long, default_value = "10")]
    count: u32,

    #[arg(long, default_value = "255")]
    alpha_cutoff: u8,

    #[arg(short, default_value = "0")]
    x: u32,

    #[arg(short, default_value = "0")]
    y: u32,

    #[arg(short, default_value = "0")]
    w: u32,

    #[arg(short, default_value = "0")]
    h: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("♥ pxlflt~");
    let args = Args::parse();

    println!("♥ setting up stream...");
    let addr = args.server;
    let mut conn = TcpStream::connect(&addr).await?;
    let (reader, mut writer) = conn.split();
    let mut buf_reader = BufReader::new(reader);

    // get screen size
    writer.write("SIZE\n".as_bytes()).await?;
    let mut buf = String::new();
    buf_reader.read_line(&mut buf).await?; // reply is ~"SIZE XXXX YYYY"
    let (width_str, height_str) = buf.split_once(" ").unwrap().1.split_once(" ").unwrap();
    let mut width: u32 = width_str.trim().parse().unwrap();
    let mut height: u32 = height_str.trim().parse().unwrap();
    println!("♥ display size: {}×{}", width, height);
    writer.shutdown().await?;

    // apply user image size if set
    if args.w != 0 {
        width = args.w
    };
    if args.h != 0 {
        height = args.h
    };

    // load image
    let image = ImageReader::open(args.image)?
        .with_guessed_format()?
        .decode()?;
    println!(
        "♥ image original size: {}×{}, resizing...",
        image.width(),
        image.height()
    );
    let mut image = image.resize_to_fill(width, height, image::imageops::FilterType::Nearest);
    println!("♥ image resized size: {}×{}", image.width(), image.height());
    let mut chunks = create_new_chunks(&mut image, args.alpha_cutoff, 10).await?;
    let mut pixel_count = 0;
    let image_size: usize = (image.width() * image.height()).try_into()?;
    for chunk in &chunks {
        pixel_count += chunk.len();
    }
    println!("♥ pixels that will be flood: {} of {} ({}%)", pixel_count, image_size, image_size / pixel_count);

    // setup streams
    let mut sharable_streams: Vec<Arc<Mutex<TcpStream>>> = Vec::new();
    for _i in 0..10 {
        let conn = TcpStream::connect(&addr).await?;
        let value = Arc::new(Mutex::new(conn));
        sharable_streams.push(value);
    }
    println!("♥ stream amount: {}", sharable_streams.len());

    // flood
    println!("♥ flooding the server with hugs, kisses and a whole lot of pixels~");
    loop {
        let _result = tokio::join!(
            create_new_chunks(&mut image, args.alpha_cutoff, 10),
            flood(&chunks[0], &sharable_streams[0], args.x, args.y),
            flood(&chunks[1], &sharable_streams[1], args.x, args.y),
            flood(&chunks[2], &sharable_streams[2], args.x, args.y),
            flood(&chunks[3], &sharable_streams[3], args.x, args.y),
            flood(&chunks[4], &sharable_streams[4], args.x, args.y),
            flood(&chunks[5], &sharable_streams[5], args.x, args.y),
            flood(&chunks[6], &sharable_streams[6], args.x, args.y),
            flood(&chunks[7], &sharable_streams[7], args.x, args.y),
            flood(&chunks[8], &sharable_streams[8], args.x, args.y),
            flood(&chunks[9], &sharable_streams[9], args.x, args.y),
        );
        if _result.0.is_ok() {
            chunks = _result.0.unwrap();
        }
    }
}

async fn create_new_chunks(
    image: &mut DynamicImage,
    alpha_cutoff: u8,
    amount: usize
) -> Result<Vec<Vec<(u32, u32, Rgba<u8>)>>> {
    let mut pixels: Vec<(u32, u32, Rgba<u8>)> = Vec::new();
    for p in image.pixels() {
        if p.2[3] >= alpha_cutoff {
            pixels.push(p);
        }
    }
    let mut rng = rand::rng();
    pixels.shuffle(&mut rng);
    let mut chunks: Vec<Vec<(u32, u32, Rgba<u8>)>> = Vec::new();
    for chunk in pixels.chunks(pixels.len() / amount) {
        chunks.push(chunk.to_vec());
    }
    Ok(chunks)
}

async fn flood<W>(
    pixels: &Vec<(u32, u32, Rgba<u8>)>,
    sharable_writer: &Arc<Mutex<W>>,
    offset_x: u32,
    offset_y: u32,
) -> Result<usize>
where
    W: AsyncWriteExt + Unpin,
{
    let mut writer = sharable_writer.lock().await;
    let mut count = 0;
    for (x, y, c) in pixels {
        // if c.0[3] != 255 { continue };
        writer
            .write(
                format!(
                    "PX {} {} {:02x}{:02x}{:02x}\n",
                    x + offset_x,
                    y + offset_y,
                    c.0[0],
                    c.0[1],
                    c.0[2]
                )
                .as_bytes(),
            )
            .await?;
        count += 1;
    }
    Ok(count)
}
