use anyhow::anyhow;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use log::{info, warn};
use ort::execution_providers::CPUExecutionProvider;
use ort::session::builder::{GraphOptimizationLevel, SessionBuilder};
use ort::session::Session;
use ort::value::Tensor;
use std::fmt::Display;
use std::ops::{BitAnd, BitOr};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Labels(u8);
pub const DRAWINGS: Labels = Labels(1u8 << 0); // - safe for work drawings (including anime)
pub const HENTAI: Labels = Labels(1u8 << 1); //- hentai and pornographic drawings
pub const NEUTRAL: Labels = Labels(1u8 << 2); //- safe for work neutral images
pub const PORN: Labels = Labels(1u8 << 3); //- pornographic images, sexual acts
pub const SEXY: Labels = Labels(1u8 << 4); //- sexually explicit images, not pornography
impl BitOr for Labels {
    type Output = Labels;

    fn bitor(self, rhs: Self) -> Self::Output {
        Labels(self.0 | rhs.0)
    }
}
impl BitAnd for Labels {
    type Output = Labels;

    fn bitand(self, rhs: Self) -> Self::Output {
        Labels(self.0 & rhs.0)
    }
}
impl Display for Labels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut v: Vec<&'static str> = vec![];
        if self.0 & DRAWINGS.0 != 0 {
            v.push("drawings");
        }
        if self.0 & HENTAI.0 != 0 {
            v.push("hentai");
        }
        if self.0 & NEUTRAL.0 != 0 {
            v.push("neutral");
        }
        if self.0 & PORN.0 != 0 {
            v.push("porn");
        }
        if self.0 & SEXY.0 != 0 {
            v.push("sexy");
        }
        write!(f, "{}", v.join(","))
    }
}
impl Labels {
    pub fn is_hentai(&self) -> bool {
        self.0 & HENTAI.0 != 0
    }
    pub fn is_porn(&self) -> bool {
        self.0 & PORN.0 != 0
    }
}
enum Instruct {
    Detect(Message),
}

const MODEL: &'static [u8] = include_bytes!("../../assets/nsfw.onnx");
#[derive(Clone)]
pub struct NSFWDetector {
    channel: tokio::sync::mpsc::Sender<Instruct>,
}
type Message = (
    DynamicImage,
    tokio::sync::oneshot::Sender<anyhow::Result<Labels>>,
);
impl NSFWDetector {
    pub fn new() -> anyhow::Result<Self> {
        if !ort::init().with_name("NSFWDetector").commit() {
            warn!("a ort environment has already been configured");
        }
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Instruct>(100);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?;
        std::thread::spawn(move || {
            let mut session: Option<Session> = None;
            let mut last = Instant::now(); // 模型最后调用时间
            let mut interval = rt.block_on(async { tokio::time::interval(Duration::from_mins(1))});
            enum Next {
                Task(Instruct),
                Exit,
                Ticker,
            }
            loop {
                let next = rt.block_on(async {
                    tokio::select! {
                        _ = interval.tick() => {
                            Next::Ticker
                        }
                        ins = receiver.recv() => {
                            match ins {
                                None => Next::Exit,
                                Some(ins) => Next::Task(ins)
                            }
                        }
                    }
                });
                match next {
                    Next::Task(Instruct::Detect((img, feedback))) => {
                        last = Instant::now();
                        if session.is_none() {
                            info!("Loading NSFW detection model");
                            match get_session() {
                                Ok(s) => {
                                    info!("Loading NSFW detection model...done");
                                    session = Some(s);
                                }
                                Err(err) => {
                                    info!("Failed to load NSFW detection model: {}", err);
                                    _ = feedback.send(Err(anyhow!("加载模型失败: {}", err)));
                                    continue;
                                }
                            }
                        }
                        if let Some(s) = session.as_mut() {
                            _ = feedback.send(detect(s, img));
                        } else {
                            _ = feedback.send(Err(anyhow!("系统错误，模型未找到")));
                        }
                    }
                    Next::Exit => {
                        info!("NSFWDetector exiting...");
                        return;
                    }
                    Next::Ticker => {
                        // 卸载模型，模型缓存 10 分钟
                        if session.is_some() && last.elapsed().as_secs_f32() > 300.0 {
                            info!("Unloading NSFW detection model");
                            session = None;
                        }
                    }
                }
            }
        });

        Ok(NSFWDetector { channel: sender })
    }
    pub async fn detect(&self, img: DynamicImage) -> anyhow::Result<Labels> {
        let (sender, receiver) = tokio::sync::oneshot::channel::<anyhow::Result<Labels>>();
        let ins: Instruct = Instruct::Detect((img, sender));
        self.channel.send(ins).await?;
        receiver.await?
    }
}

fn get_session() -> anyhow::Result<Session> {
    let ep = CPUExecutionProvider::default()
        .with_arena_allocator(true)
        .build();
    let session = SessionBuilder::new()?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|err| anyhow!("{}", err))?
        .with_intra_threads(4) // 根据 CPU 核心数调整
        .map_err(|err| anyhow!("{}", err))?
        .with_execution_providers([ep])
        .map_err(|err| anyhow!("{}", err))?
        .commit_from_memory(MODEL)?;
    Ok(session)
}
fn detect(session: &mut Session, img: DynamicImage) -> anyhow::Result<Labels> {
    // Use Triangle for better quality.
    // Use resize_exact to ensure the image fills the entire 224x224 tensor.
    // 'resize' preserves aspect ratio, which could leave parts of the tensor uninitialized (garbage data).
    let resized = img.resize_exact(224, 224, FilterType::Triangle);
    let mut input_tensor: Tensor<f32> = Tensor::new(session.allocator(), [1usize, 224, 224, 3])?; // 预分配内存，避免多次分配

    for (x, y, pixel) in resized.pixels() {
        let rgba = pixel.0;
        input_tensor[[0, y as i64, x as i64, 0]] = rgba[0] as f32 / 255.0;
        input_tensor[[0, y as i64, x as i64, 1]] = rgba[1] as f32 / 255.0;
        input_tensor[[0, y as i64, x as i64, 2]] = rgba[2] as f32 / 255.0;
    }


    let input_name = String::from(session.inputs()[0].name());

    // 1. 执行推理
    let outputs = session.run(ort::inputs![input_name=>input_tensor])?;
    let output_tensor = outputs[0].try_extract_array::<f32>()?;
    let slice = output_tensor
        .as_slice()
        .ok_or_else(|| anyhow!("Failed to get slice"))?;
    let threshold: f32 = 0.7;
    let mut labels: Labels = Labels::default();
    if slice[0] > threshold {
        labels = labels | DRAWINGS;
    }
    if slice[1] > threshold {
        labels = labels | HENTAI;
    }
    if slice[2] > threshold {
        labels = labels | NEUTRAL;
    }
    if slice[3] > threshold {
        labels = labels | PORN;
    }
    if slice[4] > threshold {
        labels = labels | SEXY;
    }
    Ok(labels)
}
