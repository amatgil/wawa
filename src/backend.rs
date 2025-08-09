//! Reimplementation of WebBackend (from uiua/pad/editor/src/lib.rs) for use in Native

use std::{
    any::Any,
    borrow::{Borrow, BorrowMut},
    collections::{HashMap, HashSet},
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};

use serenity::futures::future::join_all;
use tokio::task::spawn_local;
use uiua::{now, GitTarget, Report, SysBackend, EXAMPLE_TXT, EXAMPLE_UA};

static START_TIME: OnceLock<f64> = OnceLock::new();

#[derive(Debug)]
pub struct NativisedWebBackend {
    pub stdout: Mutex<Vec<OutputItem>>,
    pub stderr: Mutex<String>,
    pub trace: Mutex<String>,
    pub files: Mutex<HashMap<PathBuf, Vec<u8>>>,
    pub git_cache: Mutex<HashMap<String, Result<String, String>>>,
    pub git_working: Mutex<HashSet<String>>,
}

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
            git_cache: Default::default(),
            git_working: Default::default(),
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
    fn load_git_module(&self, original_url: &str, target: GitTarget) -> Result<PathBuf, String> {
        type Cache = HashMap<String, Result<String, String>>;
        type Working = HashSet<String>;
        type Files = HashMap<PathBuf, Vec<u8>>;

        fn cache_url(cache: &mut Cache, url: &str, res: Result<String, String>) {
            cache.borrow_mut().insert(url.into(), res);
        }

        fn mark_working(working: &mut Working, url: &str) {
            working.borrow_mut().insert(url.into());
        }

        fn unmark_working(working: &mut Working, url: &str) {
            working.borrow_mut().remove(url);
        }

        pub fn drop_file(files: &mut Files, path: PathBuf, contents: Vec<u8>) {
            files.borrow_mut().insert(path, contents);
        }

        pub fn delete_file(files: &mut Files, path: &PathBuf) {
            files.borrow_mut().remove(path);
        }

        let mut cache = self.git_cache.lock().expect("poisoned thread");
        let mut working = self.git_working.lock().expect("poisoned thread");

        if working.borrow().contains(original_url) {
            return Err("Waiting for module, try running again in a moment...".into());
        }

        match target {
            GitTarget::Default => {}
            GitTarget::Branch(_) => {
                return Err("Git branch specification is not supported in the wawaing".into())
            }
            GitTarget::Commit(_) => {
                return Err("Git commit specification is not supported in the wawaing".into())
            }
        }
        let (repo_owner, repo_name, path) = {
            let mut parts = original_url.rsplitn(3, '/');
            let repo_name = parts.next().ok_or("Invalid git url")?;
            let repo_owner = parts.next().ok_or("Invalid git url")?;
            let path = Path::new("uiua-modules")
                .join(repo_owner)
                .join(repo_name)
                .join("lib.ua");

            (repo_owner.to_string(), repo_name.to_string(), path)
        };
        let mut files = self.files.lock().expect("poisoned thread");

        if files.borrow().contains_key(&path) {
            return Ok(path);
        }

        let mut url = original_url
            .trim_end_matches('/')
            .replace("www.", "")
            .replace("github.com", "raw.githubusercontent.com")
            .replace("src/branch/master", "raw/branch/master");

        if !url.ends_with(".ua") {
            url = format!("{url}/main/lib.ua");
        }

        let res = if let Some(res) = cache.borrow().get(&url) {
            Some(res.clone())
        } else if original_url.contains("github.com") && url.ends_with("/lib.ua") {
            mark_working(&mut working, original_url);
            let original_url = original_url.to_string();

            spawn_local(async move {
                let tree_res = fetch(&format!(
                    "https://api.github.com\
                        /repos/{repo_owner}/{repo_name}/git/trees/main?recursive=1",
                ))
                .await;

                match tree_res {
                    Err(_) => {
                        cache_url(&mut cache, &url, tree_res);
                        unmark_working(&mut working, &original_url);
                        return;
                    }
                    Ok(_) => {
                        let tree = tree_res.unwrap();
                        let tree: serde_json::Value = serde_json::from_str(&tree).unwrap();
                        let tree = tree.get("tree").unwrap().as_array().unwrap();
                        let paths = tree
                            .iter()
                            .filter_map(|entry| {
                                let path = entry.get("path")?.as_str()?;
                                if path.ends_with(".ua") {
                                    Some(path.to_string())
                                } else {
                                    None
                                }
                            })
                            .collect::<HashSet<_>>();

                        if !paths.contains("lib.ua") {
                            cache_url(&mut cache, &url, Err("lib.ua not found".into()));
                            unmark_working(&mut working, &original_url);
                            return;
                        }

                        let results = join_all(paths.iter().map(|path| {
                            let repo_owner = repo_owner.clone();
                            let repo_name = repo_name.clone();
                            async move {
                                let fetch_url = format!(
                                    "https://raw.githubusercontent.com\
                                        /{repo_owner}/{repo_name}/main/{path}",
                                );
                                let internal_path = Path::new("uiua-modules")
                                    .join(repo_owner)
                                    .join(repo_name)
                                    .join(path.clone());

                                (path, internal_path, fetch(fetch_url.as_str()).await)
                            }
                        }))
                        .await;

                        for (original_path, internal_path, res) in results {
                            if original_path.eq("lib.ua") {
                                cache_url(&mut cache, &url, res.clone());
                            }

                            if let Ok(text) = res {
                                let contents = text.as_bytes().to_vec();
                                drop_file(&mut files, internal_path.clone(), contents);
                            }
                        }
                    }
                }

                unmark_working(&mut working, &original_url);
            });
            None
        } else {
            mark_working(&mut working, original_url);
            let original_url = original_url.to_string();
            spawn_local(async move {
                let res = fetch(&url).await;
                cache_url(&mut cache, &url, res);
                unmark_working(&mut working, &original_url);
            });
            None
        };

        match res {
            Some(Ok(text)) => {
                let contents = text.as_bytes().to_vec();
                drop_file(&mut files, path.clone(), contents);
                Ok(path)
            }
            Some(Err(err)) => Err(err),
            None => Err("Waiting for module, try running again in a moment...".into()),
        }
    }
}
pub async fn fetch(url: &str) -> Result<String, String> {
    todo!()
    /*
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
    }*/
}
