// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use anyhow::{anyhow, Result};
use clap::ValueEnum;

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[clap(rename_all = "lowercase")]
pub enum ContentEncoding {
    PlainText,
    // TODO(giac): Enable GZIP once decided what to do with the encoding.
    // Gzip,
}

impl fmt::Display for ContentEncoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ContentEncoding::PlainText => write!(f, "plaintext"),
            // ContentEncoding::Gzip => write!(f, "gzip"),
        }
    }
}

impl TryFrom<&str> for ContentEncoding {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "plaintext" => Ok(ContentEncoding::PlainText),
            // "gzip" => Ok(ContentEncoding::Gzip),
            _ => Err(anyhow!("Invalid content encoding string: {value}")),
        }
    }
}

impl TryFrom<String> for ContentEncoding {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Content types for content of a page
/// The list is generated starting from
/// <https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types>
#[derive(Debug, PartialEq, Eq, Clone, Ord, PartialOrd)]
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
    pub fn try_from_extension(ext: &str) -> Result<Self> {
        match ext {
            "aac" => Ok(ContentType::AudioAac),
            "abw" => Ok(ContentType::ApplicationXabiword),
            "apng" => Ok(ContentType::ImageApng),
            "arc" => Ok(ContentType::ApplicationXfreearc),
            "avif" => Ok(ContentType::ImageAvif),
            "avi" => Ok(ContentType::VideoXmsvideo),
            "azw" => Ok(ContentType::ApplicationVndamazonebook),
            "bin" => Ok(ContentType::ApplicationOctetstream),
            "bmp" => Ok(ContentType::ImageBmp),
            "bz" => Ok(ContentType::ApplicationXbzip),
            "bz2" => Ok(ContentType::ApplicationXbzip2),
            "cda" => Ok(ContentType::ApplicationXcdf),
            "csh" => Ok(ContentType::ApplicationXcsh),
            "css" => Ok(ContentType::TextCss),
            "csv" => Ok(ContentType::TextCsv),
            "doc" => Ok(ContentType::ApplicationMsword),
            "docx" => {
                Ok(ContentType::ApplicationVndopenxmlformatsofficedocumentwordprocessingmldocument)
            }
            "eot" => Ok(ContentType::ApplicationVndmsfontobject),
            "epub" => Ok(ContentType::ApplicationEpubzip),
            "gz" => Ok(ContentType::ApplicationGzip),
            "gif" => Ok(ContentType::ImageGif),
            "htm" => Ok(ContentType::TextHtml),
            "html" => Ok(ContentType::TextHtml),
            "ico" => Ok(ContentType::ImageVndmicrosofticon),
            "ics" => Ok(ContentType::TextCalendar),
            "jar" => Ok(ContentType::ApplicationJavaarchive),
            "jpeg" => Ok(ContentType::ImageJpeg),
            "jpg" => Ok(ContentType::ImageJpeg),
            "js" => Ok(ContentType::TextJavascript),
            "json" => Ok(ContentType::ApplicationJson),
            "jsonld" => Ok(ContentType::ApplicationLdjson),
            "mid" => Ok(ContentType::AudioMidi),
            "midi" => Ok(ContentType::AudioMidi),
            "mjs" => Ok(ContentType::TextJavascript),
            "mp3" => Ok(ContentType::AudioMpeg),
            "mp4" => Ok(ContentType::VideoMp4),
            "mpeg" => Ok(ContentType::VideoMpeg),
            "mpkg" => Ok(ContentType::ApplicationVndappleinstallerxml),
            "odp" => Ok(ContentType::ApplicationVndoasisopendocumentpresentation),
            "ods" => Ok(ContentType::ApplicationVndoasisopendocumentspreadsheet),
            "odt" => Ok(ContentType::ApplicationVndoasisopendocumenttext),
            "oga" => Ok(ContentType::AudioOgg),
            "ogv" => Ok(ContentType::VideoOgg),
            "ogg" => Ok(ContentType::VideoOgg),
            "ogx" => Ok(ContentType::ApplicationOgg),
            "opus" => Ok(ContentType::AudioOpus),
            "otf" => Ok(ContentType::FontOtf),
            "png" => Ok(ContentType::ImagePng),
            "pdf" => Ok(ContentType::ApplicationPdf),
            "php" => Ok(ContentType::ApplicationXhttpdphp),
            "ppt" => Ok(ContentType::ApplicationVndmspowerpoint),
            "pptx" => Ok(
                ContentType::ApplicationVndopenxmlformatsofficedocumentpresentationmlpresentation,
            ),
            "rar" => Ok(ContentType::ApplicationVndrar),
            "rtf" => Ok(ContentType::ApplicationRtf),
            "sh" => Ok(ContentType::ApplicationXsh),
            "svg" => Ok(ContentType::ImageSvgxml),
            "tar" => Ok(ContentType::ApplicationXtar),
            "tif" => Ok(ContentType::ImageTiff),
            "tiff" => Ok(ContentType::ImageTiff),
            "ts" => Ok(ContentType::VideoMp2t),
            "ttf" => Ok(ContentType::FontTtf),
            "txt" => Ok(ContentType::TextPlain),
            "vsd" => Ok(ContentType::ApplicationVndvisio),
            "wav" => Ok(ContentType::AudioWav),
            "weba" => Ok(ContentType::AudioWebm),
            "webm" => Ok(ContentType::VideoWebm),
            "webp" => Ok(ContentType::ImageWebp),
            "woff" => Ok(ContentType::FontWoff),
            "woff2" => Ok(ContentType::FontWoff2),
            "xhtml" => Ok(ContentType::ApplicationXhtmlxml),
            "xls" => Ok(ContentType::ApplicationVndmsexcel),
            "xlsx" => Ok(ContentType::ApplicationVndopenxmlformatsofficedocumentspreadsheetmlsheet),
            "xml" => Ok(ContentType::ApplicationXml),
            "xul" => Ok(ContentType::ApplicationVndmozillaxulxml),
            "zip" => Ok(ContentType::ApplicationZip),
            "7z" => Ok(ContentType::ApplicationX7zcompressed),
            _ => Err(anyhow!(
                "the content type for extension `{}` is currently not supported",
                ext
            )),
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

impl TryFrom<&str> for ContentType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "audio/aac" => Ok(ContentType::AudioAac),
            "application/x-abiword" => Ok(ContentType::ApplicationXabiword),
            "image/apng" => Ok(ContentType::ImageApng),
            "application/x-freearc" => Ok(ContentType::ApplicationXfreearc),
            "image/avif" => Ok(ContentType::ImageAvif),
            "video/x-msvideo" => Ok(ContentType::VideoXmsvideo),
            "application/vnd.amazon.ebook" => Ok(ContentType::ApplicationVndamazonebook),
            "application/octet-stream" => Ok(ContentType::ApplicationOctetstream),
            "image/bmp" => Ok(ContentType::ImageBmp),
            "application/x-bzip" => Ok(ContentType::ApplicationXbzip),
            "application/x-bzip2" => Ok(ContentType::ApplicationXbzip2),
            "application/x-cdf" => Ok(ContentType::ApplicationXcdf),
            "application/x-csh" => Ok(ContentType::ApplicationXcsh),
            "text/css" => Ok(ContentType::TextCss),
            "text/csv" => Ok(ContentType::TextCsv),
            "application/msword" => Ok(ContentType::ApplicationMsword),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                Ok(ContentType::ApplicationVndopenxmlformatsofficedocumentwordprocessingmldocument)
            }
            "application/vnd.ms-fontobject" => Ok(ContentType::ApplicationVndmsfontobject),
            "application/epub+zip" => Ok(ContentType::ApplicationEpubzip),
            "application/gzip" => Ok(ContentType::ApplicationGzip),
            "image/gif" => Ok(ContentType::ImageGif),
            "text/html" => Ok(ContentType::TextHtml),
            "image/vnd.microsoft.icon" => Ok(ContentType::ImageVndmicrosofticon),
            "text/calendar" => Ok(ContentType::TextCalendar),
            "application/java-archive" => Ok(ContentType::ApplicationJavaarchive),
            "image/jpeg" => Ok(ContentType::ImageJpeg),
            "text/javascript" => Ok(ContentType::TextJavascript),
            "application/json" => Ok(ContentType::ApplicationJson),
            "application/ld+json" => Ok(ContentType::ApplicationLdjson),
            "audio/midi" => Ok(ContentType::AudioMidi),
            "audio/mpeg" => Ok(ContentType::AudioMpeg),
            "video/mp4" => Ok(ContentType::VideoMp4),
            "video/mpeg" => Ok(ContentType::VideoMpeg),
            "application/vnd.apple.installer+xml" => {
                Ok(ContentType::ApplicationVndappleinstallerxml)
            }

            "application/vnd.oasis.opendocument.presentation" => {
                Ok(ContentType::ApplicationVndoasisopendocumentpresentation)
            }
            "application/vnd.oasis.opendocument.spreadsheet" => {
                Ok(ContentType::ApplicationVndoasisopendocumentspreadsheet)
            }
            "application/vnd.oasis.opendocument.text" => {
                Ok(ContentType::ApplicationVndoasisopendocumenttext)
            }
            "audio/ogg" => Ok(ContentType::AudioOgg),
            "video/ogg" => Ok(ContentType::VideoOgg),
            "application/ogg" => Ok(ContentType::ApplicationOgg),
            "audio/opus" => Ok(ContentType::AudioOpus),
            "font/otf" => Ok(ContentType::FontOtf),
            "image/png" => Ok(ContentType::ImagePng),
            "application/pdf" => Ok(ContentType::ApplicationPdf),
            "application/x-httpd-php" => Ok(ContentType::ApplicationXhttpdphp),
            "application/vnd.ms-powerpoint" => Ok(ContentType::ApplicationVndmspowerpoint),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => Ok(
                ContentType::ApplicationVndopenxmlformatsofficedocumentpresentationmlpresentation,
            ),
            "application/vnd.rar" => Ok(ContentType::ApplicationVndrar),
            "application/rtf" => Ok(ContentType::ApplicationRtf),
            "application/x-sh" => Ok(ContentType::ApplicationXsh),
            "image/svg+xml" => Ok(ContentType::ImageSvgxml),
            "application/x-tar" => Ok(ContentType::ApplicationXtar),
            "image/tiff" => Ok(ContentType::ImageTiff),
            "video/mp2t" => Ok(ContentType::VideoMp2t),
            "font/ttf" => Ok(ContentType::FontTtf),
            "text/plain" => Ok(ContentType::TextPlain),
            "application/vnd.visio" => Ok(ContentType::ApplicationVndvisio),
            "audio/wav" => Ok(ContentType::AudioWav),
            "audio/webm" => Ok(ContentType::AudioWebm),
            "video/webm" => Ok(ContentType::VideoWebm),
            "image/webp" => Ok(ContentType::ImageWebp),
            "font/woff" => Ok(ContentType::FontWoff),
            "font/woff2" => Ok(ContentType::FontWoff2),
            "application/xhtml+xml" => Ok(ContentType::ApplicationXhtmlxml),
            "application/vnd.ms-excel" => Ok(ContentType::ApplicationVndmsexcel),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
                Ok(ContentType::ApplicationVndopenxmlformatsofficedocumentspreadsheetmlsheet)
            }
            "application/xml" => Ok(ContentType::ApplicationXml),
            "application/vnd.mozilla.xul+xml" => Ok(ContentType::ApplicationVndmozillaxulxml),
            "application/zip" => Ok(ContentType::ApplicationZip),
            "application/x-7z-compressed" => Ok(ContentType::ApplicationX7zcompressed),
            _ => Err(anyhow!("Invalid conversion to content type")),
        }
    }
}

impl TryFrom<String> for ContentType {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}
