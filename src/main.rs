use anyhow::{bail, Result};
use chrono::{Utc};
use encoding_rs;
use std::error::Error;
use std::fs;
use std::io::{Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use structopt::{clap, StructOpt};
winrt::import!(
    dependencies
        os
    types
        windows::system::Launcher
);

mod memo;
mod tui;
mod gui;

#[derive(Debug, StructOpt)]
#[structopt(name = "MenMa")]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
pub struct Opt {
    #[structopt(subcommand)]
    pub sub: Sub,
}

#[derive(Debug, StructOpt)]
pub enum Sub {
    #[structopt(name = "list", about = "view list")]
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    List {
        #[structopt(short = "t", long = "tags")]
        tags: Option<Vec<String>>,
    },
    #[structopt(name = "add", about = "add memo")]
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    Add {
        #[structopt(short = "T", long = "title")]
        title: String,
        #[structopt(short = "t", long = "tags")]
        tags: Option<Vec<String>>,
    },
    #[structopt(name = "setpath", about = "set path of memo exist directory")]
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    SetPath {
        #[structopt(short = "p", long = "path")]
        path: PathBuf,
    },
    #[structopt(name = "todo", about = "open todo.txt")]
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    Todo {},
    #[structopt(name = "gui", about = "launch gui mode")]
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    GUI {},
}

fn main() -> Result<()> {
    let args = Opt::from_args();
    println!("{:?}", args);

    let lst_memo = create_memo_list();

    match args.sub {
        Sub::List { tags } => {
            match tags {
                Some(tags) => loop {
                    let lst_memo_include_thesetags: Vec<memo::Memo> =
                        if tags.iter().any(|x| x.contains("all")) {
                            lst_memo.clone()
                        } else {
                            lst_memo
                                .iter()
                                .filter(|memo| is_include_these_tags(&tags, memo.get_tags()))
                                .cloned()
                                .collect()
                        };
                    tui::launch_tui(&lst_memo_include_thesetags).unwrap();
                },
                None => {
                    bail!("tag value is incorrect. please input valid value.")
                }
            }
        }
        Sub::Add { title, tags } => {
            let path = Path::new("E:/memo/");
            let filename = Utc::now().format("%y%m%d_").to_string() + &title + ".md";

            // 複数回実行した場合上書きされる
            let mut file = match fs::File::create(path.to_str().unwrap().to_string() + &filename) {
                Err(why) => panic!("Couldn't create {}", why),
                Ok(file) => file,
            };

            let mut tags_out: String = String::new();
            match tags {
                Some(tags) => {
                    for tag in tags {
                        tags_out += &(format!("#{} ", tag));
                    }
                }
                None => {}
            };

            let contents = format!(" <!---\n tags: {}\n --->\n", tags_out);
            match file.write_all(contents.as_bytes()) {
                Err(why) => panic!("Error:{}", why),
                Ok(_) => println!("finished"),
            }

            launch_file(&(path.to_str().unwrap().to_string() + &filename)).unwrap();
            Ok(())
        }
        Sub::SetPath { path: _ } => {
            bail!("this function is not implement;")
        }
        Sub::Todo {} => {
            launch_file("E:/memo/todo.md").unwrap();
            Ok(())
        }
        Sub::GUI {} => {
            let app = gui::TemplateApp::default();
            let native_options = eframe::NativeOptions::default();
            eframe::run_native(Box::new(app), native_options); 
        }
    }
}

/// 渡されたpathに存在するmdファイルをメモとして返します。
fn create_memo_list() -> Vec<memo::Memo> {
    // TODO:ファイル読み込み
    let directory = read_dir("E:/memo").unwrap();

    let files = directory.into_iter().filter(|file| file.is_file() );
    let files_md = files.filter(|file| "md" == file.extension().unwrap().to_str().unwrap() );

    files_md.filter_map(|file| create_memo_from_file(&file)).collect()
}

fn create_memo_from_file(file: &PathBuf) -> Option<memo::Memo> {
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(_) =>  {
            let s = fs::read(&file).unwrap();
            let (res, _, _) = encoding_rs::SHIFT_JIS.decode(&s);
            res.into_owned()
        }
    };

    let lines = text.lines();

    lines.into_iter().find_map(|line| {
        match get_tags_by_line(line.to_string()) {
            Some(tags) => {
                Some(memo::Memo::new(
                    file.to_str().unwrap().replace("\\", "/").to_string(),
                    tags,
                ))
            },
            None => None,
        }
    })

}

fn get_tags_by_line(mut line: String) -> Option<Vec<String>> {
    match line.contains("tags") {
        true => {
            line.retain(|x| x != ' ');

            let mut tags: Vec<&str> = line.split('#').collect();
            tags.retain(|x| !x.contains("tags:"));
            Some(tags.iter().map(|x| x.to_string()).collect())
        }
        false => {
            None
        }
    }
}

pub fn read_dir(path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let dir = fs::read_dir(path)?;
    let mut files: Vec<PathBuf> = Vec::new();
    for item in dir.into_iter() {
        files.push(item?.path());
    }
    Ok(files)
}

fn is_include_these_tags(tags: &Vec<String>, tags_memo: &Vec<String>) -> bool {
    let mut tags_dummy = tags.clone();
    tags_dummy.retain(|tag| tags_memo.iter().all(|tag_memo| !tag.contains(tag_memo)));

    tags_dummy.is_empty()
}

fn launch_file(path: &str) -> winrt::Result<()> {
    //assert!(env::set_current_dir(&Path::new("C:/Users/user/Documents/memo")).is_ok());
    let path = path.replace("/", "\\").to_string();
    println!("{}", path);
    Command::new("Code.exe")
        .arg(path)
        .spawn()
        .expect("failed to open memo");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn is_include_these_tags_test() {
        assert_eq!(
            is_include_these_tags(
                &vec!["foo".to_string(), "bar".to_string()],
                &vec!["foo".to_string(), "bar".to_string()]
            ),
            true
        );
        assert_eq!(
            is_include_these_tags(
                &vec!["foo".to_string(), "bar".to_string()],
                &vec!["foo".to_string()]
            ),
            false
        );
        assert_eq!(
            is_include_these_tags(
                &vec!["foo".to_string()],
                &vec!["foo".to_string(), "bar".to_string()]
            ),
            true
        );
    }
}
