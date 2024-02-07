use std::fmt;

use clap::ValueEnum;

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq, Eq)]
#[clap(rename_all = "lowercase")]
pub enum ContentEncoding {
    PlainText,
    Gzip,
}

impl fmt::Display for ContentEncoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ContentEncoding::PlainText => write!(f, "plaintext"),
            ContentEncoding::Gzip => write!(f, "gzip"),
        }
    }
}

/// Content types for content of a page
/// The list is generated starting from
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types
#[derive(Debug, PartialEq, Eq)]
pub enum ContentType {
    AudioAac,
    ApplicationXabiword,
    ImageApng,
    ApplicationXfreearc,
    ImageAvif,
    VideoXmsvideo,
    ApplicationVndamazonebook,
    ApplicationOctetstream,
    ImageBmp,
    ApplicationXbzip,
    ApplicationXbzip2,
    ApplicationXcdf,
    ApplicationXcsh,
    TextCss,
    TextCsv,
    ApplicationMsword,
    ApplicationVndopenxmlformatsofficedocumentwordprocessingmldocument,
    ApplicationVndmsfontobject,
    ApplicationEpubzip,
    ApplicationGzip,
    ImageGif,
    TextHtml,
    ImageVndmicrosofticon,
    TextCalendar,
    ApplicationJavaarchive,
    ImageJpeg,
    TextJavascript,
    ApplicationJson,
    ApplicationLdjson,
    AudioMidi,
    AudioMpeg,
    VideoMp4,
    VideoMpeg,
    ApplicationVndappleinstallerxml,
    ApplicationVndoasisopendocumentpresentation,
    ApplicationVndoasisopendocumentspreadsheet,
    ApplicationVndoasisopendocumenttext,
    AudioOgg,
    VideoOgg,
    ApplicationOgg,
    AudioOpus,
    FontOtf,
    ImagePng,
    ApplicationPdf,
    ApplicationXhttpdphp,
    ApplicationVndmspowerpoint,
    ApplicationVndopenxmlformatsofficedocumentpresentationmlpresentation,
    ApplicationVndrar,
    ApplicationRtf,
    ApplicationXsh,
    ImageSvgxml,
    ApplicationXtar,
    ImageTiff,
    VideoMp2t,
    FontTtf,
    TextPlain,
    ApplicationVndvisio,
    AudioWav,
    AudioWebm,
    VideoWebm,
    ImageWebp,
    FontWoff,
    FontWoff2,
    ApplicationXhtmlxml,
    ApplicationVndmsexcel,
    ApplicationVndopenxmlformatsofficedocumentspreadsheetmlsheet,
    ApplicationXml,
    ApplicationVndmozillaxulxml,
    ApplicationZip,
    ApplicationX7zcompressed,
}

impl ContentType {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "aac" => ContentType::AudioAac,
            "abw" => ContentType::ApplicationXabiword,
            "apng" => ContentType::ImageApng,
            "arc" => ContentType::ApplicationXfreearc,
            "avif" => ContentType::ImageAvif,
            "avi" => ContentType::VideoXmsvideo,
            "azw" => ContentType::ApplicationVndamazonebook,
            "bin" => ContentType::ApplicationOctetstream,
            "bmp" => ContentType::ImageBmp,
            "bz" => ContentType::ApplicationXbzip,
            "bz2" => ContentType::ApplicationXbzip2,
            "cda" => ContentType::ApplicationXcdf,
            "csh" => ContentType::ApplicationXcsh,
            "css" => ContentType::TextCss,
            "csv" => ContentType::TextCsv,
            "doc" => ContentType::ApplicationMsword,
            "docx" => {
                ContentType::ApplicationVndopenxmlformatsofficedocumentwordprocessingmldocument
            }
            "eot" => ContentType::ApplicationVndmsfontobject,
            "epub" => ContentType::ApplicationEpubzip,
            "gz" => ContentType::ApplicationGzip,
            "gif" => ContentType::ImageGif,
            "htm" => ContentType::TextHtml,
            "html" => ContentType::TextHtml,
            "ico" => ContentType::ImageVndmicrosofticon,
            "ics" => ContentType::TextCalendar,
            "jar" => ContentType::ApplicationJavaarchive,
            "jpeg" => ContentType::ImageJpeg,
            "jpg" => ContentType::ImageJpeg,
            "js" => ContentType::TextJavascript,
            "json" => ContentType::ApplicationJson,
            "jsonld" => ContentType::ApplicationLdjson,
            "mid" => ContentType::AudioMidi,
            "midi" => ContentType::AudioMidi,
            "mjs" => ContentType::TextJavascript,
            "mp3" => ContentType::AudioMpeg,
            "mp4" => ContentType::VideoMp4,
            "mpeg" => ContentType::VideoMpeg,
            "mpkg" => ContentType::ApplicationVndappleinstallerxml,
            "odp" => ContentType::ApplicationVndoasisopendocumentpresentation,
            "ods" => ContentType::ApplicationVndoasisopendocumentspreadsheet,
            "odt" => ContentType::ApplicationVndoasisopendocumenttext,
            "oga" => ContentType::AudioOgg,
            "ogv" => ContentType::VideoOgg,
            "ogx" => ContentType::ApplicationOgg,
            "opus" => ContentType::AudioOpus,
            "otf" => ContentType::FontOtf,
            "png" => ContentType::ImagePng,
            "pdf" => ContentType::ApplicationPdf,
            "php" => ContentType::ApplicationXhttpdphp,
            "ppt" => ContentType::ApplicationVndmspowerpoint,
            "pptx" => {
                ContentType::ApplicationVndopenxmlformatsofficedocumentpresentationmlpresentation
            }
            "rar" => ContentType::ApplicationVndrar,
            "rtf" => ContentType::ApplicationRtf,
            "sh" => ContentType::ApplicationXsh,
            "svg" => ContentType::ImageSvgxml,
            "tar" => ContentType::ApplicationXtar,
            "tif" => ContentType::ImageTiff,
            "tiff" => ContentType::ImageTiff,
            "ts" => ContentType::VideoMp2t,
            "ttf" => ContentType::FontTtf,
            "txt" => ContentType::TextPlain,
            "vsd" => ContentType::ApplicationVndvisio,
            "wav" => ContentType::AudioWav,
            "weba" => ContentType::AudioWebm,
            "webm" => ContentType::VideoWebm,
            "webp" => ContentType::ImageWebp,
            "woff" => ContentType::FontWoff,
            "woff2" => ContentType::FontWoff2,
            "xhtml" => ContentType::ApplicationXhtmlxml,
            "xls" => ContentType::ApplicationVndmsexcel,
            "xlsx" => ContentType::ApplicationVndopenxmlformatsofficedocumentspreadsheetmlsheet,
            "xml" => ContentType::ApplicationXml,
            "xul" => ContentType::ApplicationVndmozillaxulxml,
            "zip" => ContentType::ApplicationZip,
            "7z" => ContentType::ApplicationX7zcompressed,
            _ => panic!("Unknown extension {}", ext),
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ContentType::AudioAac => write!(f, "audio/aac"),
            ContentType::ApplicationXabiword => write!(f, "application/x-abiword"),
            ContentType::ImageApng => write!(f, "image/apng"),
            ContentType::ApplicationXfreearc => write!(f, "application/x-freearc"),
            ContentType::ImageAvif => write!(f, "image/avif"),
            ContentType::VideoXmsvideo => write!(f, "video/x-msvideo"),
            ContentType::ApplicationVndamazonebook => write!(f, "application/vnd.amazon.ebook"),
            ContentType::ApplicationOctetstream => write!(f, "application/octet-stream"),
            ContentType::ImageBmp => write!(f, "image/bmp"),
            ContentType::ApplicationXbzip => write!(f, "application/x-bzip"),
            ContentType::ApplicationXbzip2 => write!(f, "application/x-bzip2"),
            ContentType::ApplicationXcdf => write!(f, "application/x-cdf"),
            ContentType::ApplicationXcsh => write!(f, "application/x-csh"),
            ContentType::TextCss => write!(f, "text/css"),
            ContentType::TextCsv => write!(f, "text/csv"),
            ContentType::ApplicationMsword => write!(f, "application/msword"),
            ContentType::ApplicationVndopenxmlformatsofficedocumentwordprocessingmldocument => {
                write!(
                    f,
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                )
            }
            ContentType::ApplicationVndmsfontobject => write!(f, "application/vnd.ms-fontobject"),
            ContentType::ApplicationEpubzip => write!(f, "application/epub+zip"),
            ContentType::ApplicationGzip => write!(f, "application/gzip"),
            ContentType::ImageGif => write!(f, "image/gif"),
            ContentType::TextHtml => write!(f, "text/html"),
            ContentType::ImageVndmicrosofticon => write!(f, "image/vnd.microsoft.icon"),
            ContentType::TextCalendar => write!(f, "text/calendar"),
            ContentType::ApplicationJavaarchive => write!(f, "application/java-archive"),
            ContentType::ImageJpeg => write!(f, "image/jpeg"),
            ContentType::TextJavascript => write!(f, "text/javascript"),
            ContentType::ApplicationJson => write!(f, "application/json"),
            ContentType::ApplicationLdjson => write!(f, "application/ld+json"),
            ContentType::AudioMidi => write!(f, "audio/midi"),
            ContentType::AudioMpeg => write!(f, "audio/mpeg"),
            ContentType::VideoMp4 => write!(f, "video/mp4"),
            ContentType::VideoMpeg => write!(f, "video/mpeg"),
            ContentType::ApplicationVndappleinstallerxml => {
                write!(f, "application/vnd.apple.installer+xml")
            }
            ContentType::ApplicationVndoasisopendocumentpresentation => {
                write!(f, "application/vnd.oasis.opendocument.presentation")
            }
            ContentType::ApplicationVndoasisopendocumentspreadsheet => {
                write!(f, "application/vnd.oasis.opendocument.spreadsheet")
            }
            ContentType::ApplicationVndoasisopendocumenttext => {
                write!(f, "application/vnd.oasis.opendocument.text")
            }
            ContentType::AudioOgg => write!(f, "audio/ogg"),
            ContentType::VideoOgg => write!(f, "video/ogg"),
            ContentType::ApplicationOgg => write!(f, "application/ogg"),
            ContentType::AudioOpus => write!(f, "audio/opus"),
            ContentType::FontOtf => write!(f, "font/otf"),
            ContentType::ImagePng => write!(f, "image/png"),
            ContentType::ApplicationPdf => write!(f, "application/pdf"),
            ContentType::ApplicationXhttpdphp => write!(f, "application/x-httpd-php"),
            ContentType::ApplicationVndmspowerpoint => write!(f, "application/vnd.ms-powerpoint"),
            ContentType::ApplicationVndopenxmlformatsofficedocumentpresentationmlpresentation => {
                write!(
                    f,
                    "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                )
            }
            ContentType::ApplicationVndrar => write!(f, "application/vnd.rar"),
            ContentType::ApplicationRtf => write!(f, "application/rtf"),
            ContentType::ApplicationXsh => write!(f, "application/x-sh"),
            ContentType::ImageSvgxml => write!(f, "image/svg+xml"),
            ContentType::ApplicationXtar => write!(f, "application/x-tar"),
            ContentType::ImageTiff => write!(f, "image/tiff"),
            ContentType::VideoMp2t => write!(f, "video/mp2t"),
            ContentType::FontTtf => write!(f, "font/ttf"),
            ContentType::TextPlain => write!(f, "text/plain"),
            ContentType::ApplicationVndvisio => write!(f, "application/vnd.visio"),
            ContentType::AudioWav => write!(f, "audio/wav"),
            ContentType::AudioWebm => write!(f, "audio/webm"),
            ContentType::VideoWebm => write!(f, "video/webm"),
            ContentType::ImageWebp => write!(f, "image/webp"),
            ContentType::FontWoff => write!(f, "font/woff"),
            ContentType::FontWoff2 => write!(f, "font/woff2"),
            ContentType::ApplicationXhtmlxml => write!(f, "application/xhtml+xml"),
            ContentType::ApplicationVndmsexcel => write!(f, "application/vnd.ms-excel"),
            ContentType::ApplicationVndopenxmlformatsofficedocumentspreadsheetmlsheet => write!(
                f,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            ),
            ContentType::ApplicationXml => write!(f, "application/xml"),
            ContentType::ApplicationVndmozillaxulxml => {
                write!(f, "application/vnd.mozilla.xul+xml")
            }
            ContentType::ApplicationZip => write!(f, "application/zip"),
            ContentType::ApplicationX7zcompressed => write!(f, "application/x-7z-compressed"),
        }
    }
}
