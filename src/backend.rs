//! Reimplementation of WebBackend (from uiua/pad/editor/src/lib.rs) for use in Native

use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};

use uiua::{now, Handle, Report, SysBackend, EXAMPLE_TXT, EXAMPLE_UA};

static START_TIME: OnceLock<f64> = OnceLock::new();

#[derive(Debug)]
pub struct NativisedWebBackend {
    pub stdout: Mutex<Vec<OutputItem>>,
    pub stderr: Mutex<String>,
    pub trace: Mutex<String>,
    pub files: Mutex<HashMap<PathBuf, Vec<u8>>>,
}

/*
thread_local! {
    static FILES: RefCell<HashMap<PathBuf, Vec<u8>>> = RefCell::new(
        [
            ("example.ua", EXAMPLE_UA),
            ("example.txt", EXAMPLE_TXT)
        ]
        .map(|(path, content)| (PathBuf::from(path), content.as_bytes().to_vec()))
        .into(),
    );
}*/

impl NativisedWebBackend {
    pub fn current_stdout(&self) -> Vec<OutputItem> {
        let t = self.stdout.lock().unwrap();
        t.clone()
    }
}

impl Default for NativisedWebBackend {
    fn default() -> Self {
        Self {
            stdout: Vec::new().into(),
            stderr: String::new().into(),
            trace: String::new().into(),
            files: HashMap::from([
                ("example.ua".into(), EXAMPLE_UA.bytes().collect()),
                ("example.txt".into(), EXAMPLE_TXT.bytes().collect()),
            ])
            .into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum OutputItem {
    String(String),
    Svg(String),
    Image(Vec<u8>, Option<String>),
    Gif(Vec<u8>, Option<String>),
    Audio(Vec<u8>, Option<String>),
    Report(Report),
    Faint(String),
    Classed(&'static str, String),
    Separator,
    Continuation(u32),
}

impl OutputItem {
    pub fn is_report(&self) -> bool {
        matches!(self, OutputItem::Report(_))
    }
}

impl SysBackend for NativisedWebBackend {
    fn any(&self) -> &dyn Any {
        self
    }
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn print_str_stdout(&self, s: &str) -> Result<(), String> {
        let mut stdout = self.stdout.lock().unwrap();
        let mut lines = s.lines();
        let Some(first) = lines.next() else {
            return Ok(());
        };
        if let Some(OutputItem::String(prev)) = stdout.last_mut() {
            prev.push_str(first);
        } else {
            stdout.push(OutputItem::String(first.into()));
        }
        for line in lines {
            stdout.push(OutputItem::String(line.into()));
        }
        if s.ends_with('\n') {
            stdout.push(OutputItem::String("".into()));
        }
        Ok(())
    }
    fn print_str_stderr(&self, s: &str) -> Result<(), String> {
        self.stderr.lock().unwrap().push_str(s);
        Ok(())
    }
    fn print_str_trace(&self, s: &str) {
        self.trace.lock().unwrap().push_str(s);
    }
    fn show_image(&self, image: image::DynamicImage, label: Option<&str>) -> Result<(), String> {
        let mut bytes = Cursor::new(Vec::new());
        image
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to show image: {e}"))?;
        self.stdout
            .lock()
            .unwrap()
            .push(OutputItem::Image(bytes.into_inner(), label.map(Into::into)));
        Ok(())
    }
    fn show_gif(&self, gif_bytes: Vec<u8>, label: Option<&str>) -> Result<(), String> {
        (self.stdout.lock().unwrap()).push(OutputItem::Gif(gif_bytes, label.map(Into::into)));
        Ok(())
    }
    fn now(&self) -> f64 {
        *START_TIME.get_or_init(|| 0.0) + now()
    }
    fn sleep(&self, seconds: f64) -> Result<(), String> {
        std::thread::sleep(Duration::from_secs_f64(seconds));
        Ok(())
    }
    fn allow_thread_spawning(&self) -> bool {
        true
    }
    fn file_read_all(&self, path: &Path) -> Result<Vec<u8>, String> {
        let files = self
            .files
            .lock()
            .map_err(|_| "catastrophic error (reading file)".to_string())?;
        files
            .get(&path.to_owned())
            .ok_or("File did not exist, did you send the attachment?".to_string())
            .cloned()
    }
    fn file_write_all(&self, path: &Path, contents: &[u8]) -> Result<(), String> {
        let mut files = self
            .files
            .lock()
            .map_err(|_| "catastrophic error (writing file)".to_string())?;
        files.insert(path.to_owned(), contents.to_owned());
        Ok(())
    }
    fn list_dir(&self, _: &str) -> Result<Vec<String>, String> {
        let files = self
            .files
            .lock()
            .map_err(|_| "catastrophic error reading files")?;

        Ok(files
            .keys()
            .map(|p| p.to_string_lossy().to_string())
            .collect())
    }
}

/*
pub async fn fetch(url: &str) -> Result<String, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();
    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map(|s| s.as_string().unwrap())
        .map_err(|e| format!("{e:?}"))?;
    if resp.status() == 200 {
        Ok(text)
    } else {
        Err(text)
    }
}
*/
