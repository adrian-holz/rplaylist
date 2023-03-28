use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct PlayFileConfig {
    pub file: PathBuf,
}

#[derive(Debug, PartialEq)]
pub struct PlayListConfig {
    pub playlist: PathBuf,
}

#[derive(Debug, PartialEq)]
pub struct CreateListConfig {
    pub playlist: PathBuf,
    pub file: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
pub struct AddSongConfig {
    pub playlist: PathBuf,
    pub file: PathBuf,
}

#[derive(Debug, PartialEq)]
pub enum CmdConfig {
    PlayFile(PlayFileConfig),
    PlayList(PlayListConfig),
    CreateList(CreateListConfig),
    AddFile(AddSongConfig),
}

pub fn make_config(args: &[String]) -> Result<CmdConfig, &'static str> {
    if args.len() < 2 {
        return Err("not enough arguments");
    }

    match args[1].as_str() {
        "play" => make_play_config(args),
        "create" => make_create_config(args),
        "add" => make_add_config(args),
        _ => Err("invalid action")
    }
}

fn make_play_config(args: &[String]) -> Result<CmdConfig, &'static str> {
    if args.len() < 4 {
        return Err("not enough arguments");
    }

    if args[2].as_str() == "-f" || args[2].as_str() == "--file" {
        return Ok(CmdConfig::PlayFile(PlayFileConfig { file: PathBuf::from(args[3].clone()) }));
    } else if args[2].as_str() == "-p" || args[2].as_str() == "--playlist" {
        return Ok(CmdConfig::PlayList(PlayListConfig { playlist: PathBuf::from(args[3].clone()) }));
    }
    Err("invalid option")
}


fn make_create_config(args: &[String]) -> Result<CmdConfig, &'static str> {
    if args.len() < 3 {
        return Err("not enough arguments");
    }

    let mut count = 0;
    let file = if args[2].as_str() == "-f" || args[2].as_str() == "--file" {
        if args.len() < 4 {
            return Err("not enough arguments");
        }
        count += 2;
        Some(PathBuf::from(args[3].clone()))
    } else {
        None
    };

    if args.len() < 3 + count {
        return Err("not enough arguments");
    }

    Ok(CmdConfig::CreateList(CreateListConfig { playlist: PathBuf::from(args[2 + count].clone()), file }))
}

fn make_add_config(args: &[String]) -> Result<CmdConfig, &'static str> {
    if args.len() < 4 {
        return Err("not enough arguments");
    }

    Ok(CmdConfig::AddFile(AddSongConfig { playlist: PathBuf::from(args[3].clone()), file: PathBuf::from(args[2].clone()) }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_action() -> Result<(), &'static str> {
        let vals = [String::from("exe"), String::from("invalid")];
        match make_config(&vals) {
            Ok(_) => Err("Should return Err"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn valid_play_file_1() {
        let vals = [String::from("exe"), String::from("play"), String::from("-f"), String::from("some.mp3")];
        let a = make_config(&vals).expect("Should return a configuration");
        let b = CmdConfig::PlayFile(PlayFileConfig { file: PathBuf::from("some.mp3") });
        assert_eq!(a, b);
    }

    #[test]
    fn valid_play_file_2() {
        let vals = [String::from("exe"), String::from("play"), String::from("--file"), String::from("some.mp3")];
        let a = make_config(&vals).expect("Should return a configuration");
        let b = CmdConfig::PlayFile(PlayFileConfig { file: PathBuf::from("some.mp3") });
        assert_eq!(a, b);
    }

    #[test]
    fn valid_play_list_1() {
        let vals = [String::from("exe"), String::from("play"), String::from("-p"), String::from("some.playlist")];
        let a = make_config(&vals).expect("Should return a configuration");
        let b = CmdConfig::PlayList(PlayListConfig { playlist: PathBuf::from("some.playlist") });
        assert_eq!(a, b);
    }

    #[test]
    fn valid_play_list_2() {
        let vals = [String::from("exe"), String::from("play"), String::from("--playlist"), String::from("some.playlist")];
        let a = make_config(&vals).expect("Should return a configuration");
        let b = CmdConfig::PlayList(PlayListConfig { playlist: PathBuf::from("some.playlist") });
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_play_option() -> Result<(), &'static str> {
        let vals = [String::from("exe"), String::from("play"), String::from("--invalid"), String::from("some.playlist")];
        match make_config(&vals) {
            Ok(_) => Err("Should return Err"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn invalid_play_too_short() -> Result<(), &'static str> {
        let vals = [String::from("exe"), String::from("play")];
        match make_config(&vals) {
            Ok(_) => Err("Should return Err"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn valid_create_play_list() {
        let vals = [String::from("exe"), String::from("create"), String::from("some.playlist")];
        let a = make_config(&vals).expect("Should return a configuration");
        let b = CmdConfig::CreateList(CreateListConfig { playlist: PathBuf::from("some.playlist"), file: None });
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_create_too_short() -> Result<(), &'static str> {
        let vals = [String::from("exe"), String::from("create")];
        match make_config(&vals) {
            Ok(_) => Err("Should return Err"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn valid_create_with_file_1() {
        let vals = [String::from("exe"), String::from("create"), String::from("-f"),
            String::from("some.mp3"), String::from("some.playlist")];
        let a = make_config(&vals).expect("Should return a configuration");
        let b = CmdConfig::CreateList(CreateListConfig { playlist: PathBuf::from("some.playlist"), file: Some(PathBuf::from("some.mp3")) });
        assert_eq!(a, b);
    }

    #[test]
    fn valid_create_with_file_2() {
        let vals = [String::from("exe"), String::from("create"), String::from("--file"),
            String::from("some.mp3"), String::from("some.playlist")];
        let a = make_config(&vals).expect("Should return a configuration");
        let b = CmdConfig::CreateList(CreateListConfig { playlist: PathBuf::from("some.playlist"), file: Some(PathBuf::from("some.mp3")) });
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_create_with_file_too_short() -> Result<(), &'static str> {
        let vals = [String::from("exe"), String::from("create"), String::from("--file"),
            String::from("some.mp3")];
        match make_config(&vals) {
            Ok(_) => Err("Should return Err"),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn valid_add_file_1() {
        let vals = [String::from("exe"), String::from("add"), String::from("some.mp3"), String::from("some.playlist")];
        let a = make_config(&vals).expect("Should return a configuration");
        let b = CmdConfig::AddFile(AddSongConfig { playlist: PathBuf::from("some.playlist"), file: PathBuf::from("some.mp3") });
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_add_file_too_short() -> Result<(), &'static str> {
        let vals = [String::from("exe"), String::from("add"), String::from("some.mp3")];
        match make_config(&vals) {
            Ok(_) => Err("Should return Err"),
            Err(_) => Ok(()),
        }
    }
}
