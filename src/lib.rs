//! The library encodes file path separators and common directory names, producing a reversible unicode string that can be used as a filename. It's useful in the case when you want to extract data or features from any file and store them in a specific directory.
//! 
//! It replaces path chars as below:
//! 
//! - chars `\/:*?"<>|` be replaced to full width alternative chars of unicode.
//! - U+0000 be replaced to `〇`.
//! - a common directory, like home, documents, pictures, etc are replaced to a OS icon (🍎, 🐧, etc) and a directory icon (🏠, 📄, 🎨, etc).
//! - chars replacements for others be replaced to twice-sequential chars itself
//! 
//! # Examples
//! 
//! ```rust
//! use path_to_unicode_filename::*;
//!
//! assert_eq!(to_filename("/tmp/file.txt"), Ok("／tmp／file.txt".into()));
//! 
//! assert_eq!(to_filename("C:\\Users\\alice\\file.txt"), Ok("💠🏠alice＼file.txt".into()));
//! 
//! assert_eq!(to_filename("/Users/alice/Documents/file.txt"), Ok("🍎📄alice／file.txt".into()));
//! ```
//!

use std::{
    path::{
        Path,
        PathBuf,
    },
    ffi::{
        OsStr,
        OsString,
    },
    collections::{
        HashMap,
    },
    iter::{
        zip,
    },
};

use nom::{
    bytes::{
        complete::{
            tag,
            take,
            take_while1,
        },
    },
    character::{
        complete::{
            char,
            satisfy,
        },
    },
    sequence::{
        preceded,
        terminated,
        delimited,
    },
    branch::{
        alt,
    },
    combinator::{
        eof,
        map,
        recognize,
        success,
        peek,
        fail,
        verify,
    },
    multi::{
        fold_many0,
    },
    Err,
    IResult,
    Needed,
};

const POSIX_SEP: char = '/';
const WINDOWS_SEP: char = '\\';

const MAC_ICON: char = '🍎';
const LINUX_ICON: char = '🐧';
const WINDOWS_ICON: char = '💠';

const ESCAPE_TARGET_CHARS: &str = "\0\\/:*?\"<>|🍎🐧💠";
const ESCAPED_CHARS: &str = "〇＼／：＊？＂＜＞｜🍏🐤🚪";

const HOME_ICON: char = '🏠';
const MUSIC_ICON: char = '🎵';
const APP_DATA_ICON: char = '💾';
const DESKTOP_ICON: char = '🔝';
const DOCUMENTS_ICON: char = '📄';
const DOWNLOADS_ICON: char = '⏬';
const PICTURES_ICON: char = '🎨';
const VIDEOS_ICON: char = '🎥';
const DRIVE_ICON: char = '🥞';

#[derive(Debug, PartialEq)]
pub enum Error {
    CouldntEncodeToUtf8(OsString),
    ParseError(nom::error::Error<String>),
    IncompleteStream(Needed),
}

impl<T> From<Err<nom::error::Error<T>>> for Error
where
    T: ToString,
{
    fn from(err: Err<nom::error::Error<T>>) -> Self {
        match err {
            Err::Incomplete(needed) => Error::IncompleteStream(needed),
            Err::Error(err) | Err::Failure(err) => {
                Error::ParseError(nom::error::Error { input: err.input.to_string(), code: err.code })
            },
        }
    }
}

type ParseResult<'a, T = &'a str> = IResult<&'a str, T, nom::error::Error<&'a str>>;

struct Platform {
    prefix: char,
    sep: char,
    parse_sep: fn(i: &str) -> ParseResult,
    home_dir: fn(user: &str) -> String,
    parse_home_dir: fn(i: &str) -> ParseResult,
    drive_dir: fn(volume: &str) -> String,
    parse_drive_dir: fn(i: &str) -> ParseResult,
    music_dir: &'static str,
    app_data_dir: &'static str,
    desktop_dir: &'static str,
    documents_dir: &'static str,
    downloads_dir: &'static str,
    pictures_dir: &'static str,
    videos_dir: &'static str,
}

enum CommonRootDir {
    Home(String),
    Music(String),
    AppData(String),
    Desktop(String),
    Documents(String),
    Downloads(String),
    Pictures(String),
    Videos(String),

    Drive(String),
}

impl Platform {
    fn mac() -> Self {
        Self {
            prefix: MAC_ICON,
            sep: POSIX_SEP,
            parse_sep: Self::parse_posix_sep,
            home_dir: Self::mac_home_dir,
            parse_home_dir: Self::parse_mac_home_dir,
            drive_dir: Self::mac_drive_dir,
            parse_drive_dir: Self::parse_mac_drive_dir, 
            app_data_dir: "Library/Application Support",
            ..Platform::default()
        }
    }

    fn linux() -> Self {
        Self {
            prefix: LINUX_ICON,
            sep: POSIX_SEP,
            parse_sep: Self::parse_posix_sep,
            home_dir: Self::linux_home_dir,
            parse_home_dir: Self::parse_linux_home_dir,
            drive_dir: Self::linux_drive_dir, 
            parse_drive_dir: Self::parse_linux_drive_dir, 
            app_data_dir: ".local/share",
            ..Platform::default()
        }
    }

    fn windows() -> Self {
        Self {
            prefix: WINDOWS_ICON,
            sep: WINDOWS_SEP,
            parse_sep: Self::parse_windows_sep,
            home_dir: Self::windows_home_dir,
            parse_home_dir: Self::parse_windows_home_dir,
            drive_dir: Self::windows_drive_dir, 
            parse_drive_dir: Self::parse_windows_drive_dir, 
            app_data_dir: "AppData\\Local",
            ..Platform::default()
        }
    }

    fn default() -> Self {
        Self {
            prefix: LINUX_ICON,
            sep: POSIX_SEP,
            parse_sep: Self::parse_fail,
            home_dir: Self::linux_home_dir,
            parse_home_dir: Self::parse_fail,
            drive_dir: Self::linux_drive_dir,
            parse_drive_dir: Self::parse_fail, 
            music_dir: "Music",
            app_data_dir: "AppData",
            desktop_dir: "Desktop",
            documents_dir: "Documents",
            downloads_dir: "Downloads",
            pictures_dir: "Pictures",
            videos_dir: "Videos",
        }
    }

    fn parse_filename_platform(i: &str) -> ParseResult<Self> {
        alt((
                map(char(MAC_ICON), |_| Self::mac()),
                map(char(LINUX_ICON), |_| Self::linux()),
                map(char(WINDOWS_ICON), |_| Self::windows()),
        ))(i)
    }

    fn sniff_path_platform(i: &str) -> ParseResult<Self> {
        peek(alt((
                    map(alt((Self::parse_mac_home_dir, Self::parse_mac_drive_dir)), |_| Self::mac()),
                    map(alt((Self::parse_linux_home_dir, Self::parse_linux_drive_dir)), |_| Self::linux()),
                    map(alt((Self::parse_windows_home_dir, Self::parse_windows_drive_dir)), |_| Self::windows()),
        )))(i)
    }

    fn parse_filename_prefix<'a>(&self, i: &'a str, escaper: &'a Escaper) -> ParseResult<'a, String> {
        alt((
                map(preceded(char(HOME_ICON), escaper.unescape_path_comp(self.sep)), |user| (self.home_dir)(&user)),
                map(preceded(char(MUSIC_ICON), escaper.unescape_path_comp(self.sep)), |user| format!("{}{}{}", (self.home_dir)(&user), self.sep, self.music_dir)),
                map(preceded(char(APP_DATA_ICON), escaper.unescape_path_comp(self.sep)), |user| format!("{}{}{}", (self.home_dir)(&user), self.sep, self.app_data_dir)),
                map(preceded(char(DESKTOP_ICON), escaper.unescape_path_comp(self.sep)), |user| format!("{}{}{}", (self.home_dir)(&user), self.sep, self.desktop_dir)),
                map(preceded(char(DOCUMENTS_ICON), escaper.unescape_path_comp(self.sep)), |user| format!("{}{}{}", (self.home_dir)(&user), self.sep, self.documents_dir)),
                map(preceded(char(DOWNLOADS_ICON), escaper.unescape_path_comp(self.sep)), |user| format!("{}{}{}", (self.home_dir)(&user), self.sep, self.downloads_dir)),
                map(preceded(char(PICTURES_ICON), escaper.unescape_path_comp(self.sep)), |user| format!("{}{}{}", (self.home_dir)(&user), self.sep, self.pictures_dir)),
                map(preceded(char(VIDEOS_ICON), escaper.unescape_path_comp(self.sep)), |user| format!("{}{}{}", (self.home_dir)(&user), self.sep, self.videos_dir)),
                map(preceded(char(DRIVE_ICON), escaper.unescape_path_comp(self.sep)), |volume| (self.drive_dir)(&volume)),
        ))(i)
    }

    fn parse_path_prefix<'a>(&self, i: &'a str, escaper: &'a Escaper) -> (&'a str, String) {
        use CommonRootDir::*;

        let sep = self.parse_sep;

        let (i, dir) = match (self.parse_home_dir)(i) {
            Ok((i, user)) => {
                alt((
                        map(delimited(sep, Self::tag_or_fail(self.music_dir), peek(alt((sep, eof)))), |_| Music(escaper.escape(user))),
                        map(delimited(sep, Self::tag_or_fail(self.app_data_dir), peek(alt((sep, eof)))), |_| AppData(escaper.escape(user))),
                        map(delimited(sep, Self::tag_or_fail(self.desktop_dir), peek(alt((sep, eof)))), |_| Desktop(escaper.escape(user))),
                        map(delimited(sep, Self::tag_or_fail(self.documents_dir), peek(alt((sep, eof)))), |_| Documents(escaper.escape(user))),
                        map(delimited(sep, Self::tag_or_fail(self.downloads_dir), peek(alt((sep, eof)))), |_| Downloads(escaper.escape(user))),
                        map(delimited(sep, Self::tag_or_fail(self.pictures_dir), peek(alt((sep, eof)))), |_| Pictures(escaper.escape(user))),
                        map(delimited(sep, Self::tag_or_fail(self.videos_dir), peek(alt((sep, eof)))), |_| Videos(escaper.escape(user))),
                        map(success(()), |_| Home(escaper.escape(user))),
                ))(i).expect("using success, it cannot be failed here")
            },
            Err(_) => {
                let (i, volume) = (self.parse_drive_dir)(i).expect("sniffing in advance, it cannot be failed here");
                (i, Drive(escaper.escape(volume)))
            },
        };

        (i, match dir {
            Home(user) => format!("{}{}", HOME_ICON, user),
            Music(user) => format!("{}{}", MUSIC_ICON, user),
            AppData(user) => format!("{}{}", APP_DATA_ICON, user),
            Desktop(user) => format!("{}{}", DESKTOP_ICON, user),
            Documents(user) => format!("{}{}", DOCUMENTS_ICON, user),
            Downloads(user) => format!("{}{}", DOWNLOADS_ICON, user),
            Pictures(user) => format!("{}{}", PICTURES_ICON, user),
            Videos(user) => format!("{}{}", VIDEOS_ICON, user),
            Drive(volume) => format!("{}{}", DRIVE_ICON, volume),
        })
    }

    fn tag_or_fail<'a>(name: &'a str) -> impl Fn(&'a str) -> ParseResult<'a> {
        move |i: &'a str| {
            tag(name)(i)
        }
    }

    fn mac_home_dir(user: &str) -> String {
        "/Users/".to_string() + user
    }

    fn linux_home_dir(user: &str) -> String {
        "/home/".to_string() + user
    }

    fn windows_home_dir(user: &str) -> String {
        "C:\\Users\\".to_string() + user
    }

    fn parse_mac_home_dir(i: &str) -> ParseResult {
        delimited(tag("/Users/"), Self::parse_posix_path_comp, peek(alt((Self::parse_posix_sep, eof))))(i)
    }

    fn parse_linux_home_dir(i: &str) -> ParseResult {
        delimited(tag("/home/"), Self::parse_posix_path_comp, peek(alt((Self::parse_posix_sep, eof))))(i)
    }

    fn parse_windows_home_dir(i: &str) -> ParseResult {
        delimited(tag("C:\\Users\\"), Self::parse_windows_path_comp, peek(alt((Self::parse_windows_sep, eof))))(i)
    }

    fn mac_drive_dir(volume: &str) -> String {
        "/Volumes/".to_string() + volume
    }

    fn linux_drive_dir(volume: &str) -> String {
        "/media/".to_string() + volume
    }

    fn windows_drive_dir(volume: &str) -> String {
        volume.to_string() + ":"
    }

    fn parse_mac_drive_dir(i: &str) -> ParseResult {
        delimited(tag("/Volumes/"), Self::parse_posix_path_comp, peek(alt((Self::parse_posix_sep, eof))))(i)
    }

    fn parse_linux_drive_dir(i: &str) -> ParseResult {
        delimited(tag("/media/"), Self::parse_posix_path_comp, peek(alt((Self::parse_posix_sep, eof))))(i)
    }

    fn parse_windows_drive_dir(i: &str) -> ParseResult {
        terminated(recognize(satisfy(|c| c.is_alphabetic())), char(':'))(i)
    }

    fn parse_posix_sep(i: &str) -> ParseResult {
        recognize(char(POSIX_SEP))(i)
    }

    fn parse_posix_path_comp(i: &str) -> ParseResult {
        take_while1(|c| c != POSIX_SEP)(i)
    }

    fn parse_windows_sep(i: &str) -> ParseResult {
        recognize(char(WINDOWS_SEP))(i)
    }

    fn parse_windows_path_comp(i: &str) -> ParseResult {
        take_while1(|c| c != WINDOWS_SEP)(i)
    }

    fn parse_fail(i: &str) -> ParseResult {
        fail(i)
    }
}

struct Escaper {
    escaping_map: HashMap<char, String>,
    unescaping_map: HashMap<String, char>,
}

impl Escaper {
    fn new() -> Self {
        let mut escaping_map = HashMap::new();
        let mut unescaping_map = HashMap::new();
        for (target, escaped) in zip(ESCAPE_TARGET_CHARS.chars(), ESCAPED_CHARS.chars()) {
            escaping_map.insert(target, escaped.to_string());
            unescaping_map.insert(escaped.to_string(), target);
        }
        for c in ESCAPED_CHARS.chars() {
            let mut escaped_str = c.to_string();
            escaped_str.push(c);
            escaping_map.insert(c, escaped_str.clone());
            unescaping_map.insert(escaped_str, c);
        }
        Self {
            escaping_map,
            unescaping_map,
        }
    }

    fn escape(&self, s: &str) -> String {
        let mut r = String::new();
        for c in s.chars() {
            if let Some(escaped) = self.escaping_map.get(&c) {
                r.push_str(escaped);
            } else {
                r.push(c);
            }
        };
        r
    }

    fn unescape_char<'a>(&'a self, i: &'a str) -> ParseResult<'a, String> {
        map(alt((verify(take(2usize), |s: &str| self.unescaping_map.contains_key(s)), take(1usize))), |s: &str| {
            if let Some(c) = self.unescaping_map.get(s) {
                String::from(*c)
            } else {
                String::from(s)
            }
        })(i)
    }

    fn unescape<'a>(&'a self, i: &'a str) -> ParseResult<'a, String> {
        fold_many0(
            |i| self.unescape_char(i),
            String::new,
            |mut acc, item| {
                acc.push_str(&item);
                acc
            }
        )(i)
    }

    fn unescape_path_comp<'a>(&'a self, sep: char) -> impl FnMut(&'a str) -> ParseResult<'a, String> {
        move |i| {
            fold_many0(
                verify(|i| self.unescape_char(i), |s: &String| s.len() > 1 || s.chars().nth(0) != Some(sep)),
                String::new,
                |mut acc, item| {
                    acc.push_str(&item);
                    acc
                }
            )(i)
        }
    }
}

pub fn to_path(filename: impl AsRef<OsStr>) -> Result<PathBuf, Error> {
    let filename = filename.as_ref();
    let Some(filename) = filename.to_str() else {
        return Err(Error::CouldntEncodeToUtf8(filename.into()));
    };
    to_path_from_str(filename)
}

pub fn to_path_from_str(filename: impl AsRef<str>) -> Result<PathBuf, Error> {
    let escaper = Escaper::new();

    let i = filename.as_ref();
    let (i, prefix) = match Platform::parse_filename_platform(i) {
        Ok((i, platform)) => {
            let (i, prefix) = platform.parse_filename_prefix(i, &escaper)?;
            (i, prefix)
        },
        Err(_) => (i, "".to_string()),
    };
    let (i, path) = escaper.unescape(i).expect("it shouldn't be an error if the escaper design is correct");

    assert_eq!(i.len(), 0);

    Ok(PathBuf::from(prefix + &path))
}

pub fn to_filename(path: impl AsRef<Path>) -> Result<String, Error> {
    let path = path.as_ref();
    let path = path.as_os_str();
    let Some(path) = path.to_str() else {
        return Err(Error::CouldntEncodeToUtf8(path.into()));
    };
    Ok(to_filename_from_str(path))
}

pub fn to_filename_from_str(path: impl AsRef<str>) -> String {
    let escaper = Escaper::new();

    let i = path.as_ref();
    let (i, platform) = match Platform::sniff_path_platform(i) {
        Ok((i, platform)) => (i, Some(platform)),
        Err(_) => (i, None),
    };

    let (i, prefix) = if let Some(platform) = platform {
        let mut prefix = String::new();
        prefix.push(platform.prefix);

        let (i, p) = platform.parse_path_prefix(i, &escaper);
        prefix.push_str(&p);
        (i, prefix)
    } else {
        (i, String::new())
    };

    prefix + &escaper.escape(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ucd::Codepoint;
    use ucd::tables::misc::EastAsianWidth::*;
    use nom::{
        Needed,
        error::ErrorKind,
    };

    #[test]
    fn check_chars() {
        assert_eq!(ESCAPE_TARGET_CHARS.chars().collect::<Vec<_>>().len(), ESCAPED_CHARS.chars().collect::<Vec<_>>().len());
        for c in ESCAPED_CHARS.chars() {
            assert_explicit_width(c);
        }

        assert_explicit_width(MAC_ICON);
        assert_explicit_width(LINUX_ICON);
        assert_explicit_width(WINDOWS_ICON);

        assert_explicit_width(HOME_ICON);
        assert_explicit_width(MUSIC_ICON);
        assert_explicit_width(APP_DATA_ICON);
        assert_explicit_width(DESKTOP_ICON);
        assert_explicit_width(DOCUMENTS_ICON);
        assert_explicit_width(DOWNLOADS_ICON);
        assert_explicit_width(PICTURES_ICON);
        assert_explicit_width(VIDEOS_ICON);
        assert_explicit_width(DRIVE_ICON);
    }

    fn assert_explicit_width(c: char) {
        let w = c.east_asian_width();
        assert!(w == Narrow || w == Wide || w == HalfWidth || w == FullWidth);
    }

    #[test]
    fn it_works() {
        let pairs = [
            ("/", "／"),
            ("🍎", "🍏"),
            ("/tmp", "／tmp"),
            ("/media/disk001/file.txt", "🐧🥞disk001／file.txt"),
            ("C:\\file.txt", "💠🥞C＼file.txt"),
            ("C:\\Users\\alice\\file.txt", "💠🏠alice＼file.txt"),
            ("C:\\Users\\alice\\Music\\file.mp3", "💠🎵alice＼file.mp3"),
            ("/Users/alice/Library/Application Support", "🍎💾alice"),
            ("/home/alice/Desktop/", "🐧🔝alice／"),
            ("/home/alice/Documents/file.doc", "🐧📄alice／file.doc"),
            ("/Users/alice/Downloads/file.txt", "🍎⏬alice／file.txt"),
            ("C:\\Users\\alice\\Pictures\\file.jpg", "💠🎨alice＼file.jpg"),
            ("/home/alice/Videos/file.mp4", "🐧🎥alice／file.mp4"),
            ("/Volumes/disk001/file.txt", "🍎🥞disk001／file.txt"),
            ("platform_icon_🍎_test", "platform_icon_🍏_test"),
            ("platform_icon_🐧_test", "platform_icon_🐤_test"),
            ("platform_icon_💠_test", "platform_icon_🚪_test"),
            ("all_escape_targets_\0\\/:*?\"<>|🍎🐧💠_test", "all_escape_targets_〇＼／：＊？＂＜＞｜🍏🐤🚪_test"),
            ("all_escape_escaped_chars_〇＼／：＊？＂＜＞｜🍏🐤🚪_test", "all_escape_escaped_chars_〇〇＼＼／／：：＊＊？？＂＂＜＜＞＞｜｜🍏🍏🐤🐤🚪🚪_test"),
            ("/Volumes/disk🍎001/file.txt", "🍎🥞disk🍏001／file.txt"),
            ("/Volumes/disk🐤001/file.txt", "🍎🥞disk🐤🐤001／file.txt"),
        ];

        for (path, filename) in pairs {
            {
                let filename = OsString::from(filename);
                assert_eq!(to_path(filename).unwrap(), PathBuf::from(path));
            }
            {
                let path = PathBuf::from(path);
                assert_eq!(to_filename(path).unwrap(), filename);
            }
        }
    }

    #[test]
    fn parse_error() {
        assert_eq!(to_path("🍎invalid"), Err(Error::ParseError(nom::error::Error { input: "invalid".into(), code: ErrorKind::Char })));
    }

    #[test]
    #[cfg(any(target_os = "unix", target_os = "macos", target_os = "linux"))]
    fn parse_error_in_unix() {
        use std::os::unix::ffi::OsStringExt;
        assert_eq!(to_path(OsString::from_vec(vec![0xc3u8, 0x28u8])), Err(Error::CouldntEncodeToUtf8(OsString::from_vec(vec![0xc3u8, 0x28u8]))));
        assert_eq!(to_filename(PathBuf::from(OsString::from_vec(vec![0xc3u8, 0x28u8]))), Err(Error::CouldntEncodeToUtf8(OsString::from_vec(vec![0xc3u8, 0x28u8]))));
    }

    #[test]
    fn just_for_coverage() {
        assert_eq!(Error::from(Err::<nom::error::Error<&str>>::Incomplete(Needed::Unknown)), Error::IncompleteStream(Needed::Unknown));
        assert_eq!(
            Error::from(Err::<nom::error::Error<&str>>::Failure(nom::error::Error { input: "error", code: ErrorKind::Fail })),
            Error::ParseError(nom::error::Error { input: "error".into(), code: ErrorKind::Fail }),
        );
        assert_eq!(
            Platform::parse_fail("error"),
            Err(Err::Error(nom::error::Error { input: "error", code: ErrorKind::Fail })),
        );
        assert_eq!(
            format!("{:?}", Error::ParseError(nom::error::Error { input: "error".into(), code: ErrorKind::Fail })),
            "ParseError(Error { input: \"error\", code: Fail })".to_string(),
        );
    }
}

