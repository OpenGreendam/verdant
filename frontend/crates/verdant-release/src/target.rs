use std::{io::{Read, Seek, SeekFrom}, path::Path};
use anyhow::{Result, bail, ensure};
use serde::Serialize;
use crate::files;

pub(crate) const ORT_VERSION: &str = "1.23.2";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target { LinuxX64, LinuxArm64, WindowsX64, MacArm64 }

#[derive(Serialize)]
pub(crate) struct Runtime {
    pub target: &'static str,
    pub version: &'static str,
    pub wheel_url: &'static str,
    pub wheel_sha256: &'static str,
    pub files: Vec<RuntimeFile>,
}

#[derive(Serialize)]
pub(crate) struct RuntimeFile { pub name: &'static str, pub sha256: &'static str, pub library: bool }

impl Target {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        match value {
            "linux-x86_64" => Ok(Self::LinuxX64), "linux-aarch64" => Ok(Self::LinuxArm64),
            "windows-x86_64" => Ok(Self::WindowsX64), "macos-aarch64" => Ok(Self::MacArm64),
            _ => bail!("unsupported release target {value}"),
        }
    }
    pub(crate) fn native() -> Result<Self> {
        Self::parse(&format!("{}-{}", if cfg!(target_os = "macos") { "macos" } else { std::env::consts::OS }, std::env::consts::ARCH))
    }
    pub(crate) fn name(self) -> &'static str {
        match self { Self::LinuxX64 => "linux-x86_64", Self::LinuxArm64 => "linux-aarch64", Self::WindowsX64 => "windows-x86_64", Self::MacArm64 => "macos-aarch64" }
    }
    pub(crate) fn windows(self) -> bool { self == Self::WindowsX64 }
    pub(crate) fn mac(self) -> bool { self == Self::MacArm64 }
    pub(crate) fn extension(self) -> &'static str { if self.windows() { ".exe" } else { "" } }
    pub(crate) fn executable_dir(self) -> &'static str { if self.mac() { "Verdant.app/Contents/MacOS" } else { "bin" } }
    pub(crate) fn data_dir(self) -> &'static str { if self.mac() { "Verdant.app/Contents/Resources/share/verdant" } else { "share/verdant" } }
    pub(crate) fn docs_dir(self) -> &'static str { if self.mac() { "Verdant.app/Contents/Resources/share/doc/verdant" } else { "share/doc/verdant" } }
    pub(crate) fn program_path(self, program: &str) -> String {
        if program == "verdant-launcher" { return format!("{program}{}", self.extension()); }
        let directory = if !self.windows() && !self.mac() && matches!(program, "verdant-gui" | "verdant-plughost") { "libexec/verdant" } else { self.executable_dir() };
        format!("{directory}/{program}{}", self.extension())
    }
    pub(crate) fn library_path(self, name: &str) -> String {
        format!("{}/{name}", if self.windows() { "bin" } else if self.mac() { "Verdant.app/Contents/Frameworks" } else { "lib" })
    }
    pub(crate) fn runtime(self) -> Runtime {
        let license = "2f07c72751aed99790b8a4869cf2311df85a860b22ded05fa22803587a48922c";
        let notices = "e9e90971a8e75a9a8ac0c6412e29c1202d079998389915aa485f46c816c3b4cc";
        let (url, wheel, mut members) = match self {
            Self::LinuxX64 => (
                "https://files.pythonhosted.org/packages/ef/88/9cc25d2bafe6bc0d4d3c1db3ade98196d5b355c0b273e6a5dc09c5d5d0d5/onnxruntime-1.23.2-cp312-cp312-manylinux_2_27_x86_64.manylinux_2_28_x86_64.whl",
                "0f9b4ae77f8e3c9bee50c27bc1beede83f786fe1d52e99ac85aa8d65a01e9b77",
                vec![("libonnxruntime.so.1.23.2", "718c3fb0785d5ea06061b92c32ce4ad35be1bd4c71ae6192efe06f1ab21f0f8a", true)]),
            Self::LinuxArm64 => (
                "https://files.pythonhosted.org/packages/3f/d8/506eed9af03d86f8db4880a4c47cd0dffee973ef7e4f4cff9f1d4bcf7d22/onnxruntime-1.23.2-cp311-cp311-manylinux_2_27_aarch64.manylinux_2_28_aarch64.whl",
                "bbfd2fca76c855317568c1b36a885ddea2272c13cb0e395002c402f2360429a6",
                vec![("libonnxruntime.so.1.23.2", "ab8c4363e06ac80b3d1279ea55ebea44e906c5b131ba783ff684a067540c0e94", true)]),
            Self::WindowsX64 => (
                "https://files.pythonhosted.org/packages/3a/db/1b4a62e23183a0c3fe441782462c0ede9a2a65c6bbffb9582fab7c7a0d38/onnxruntime-1.23.2-cp311-cp311-win_amd64.whl",
                "902c756d8b633ce0dedd889b7c08459433fbcf35e9c38d1c03ddc020f0648c6e",
                vec![("onnxruntime.dll", "93457c4f17904d0ab98f3e9ac27ed153993f4b5b769337cbfd6f4c528fb0a509", true),
                    ("onnxruntime_providers_shared.dll", "056da4c9eec7a635ff79b8209b52ad1f6930528780257733955db3cdbcb59358", true)]),
            Self::MacArm64 => (
                "https://files.pythonhosted.org/packages/44/be/467b00f09061572f022ffd17e49e49e5a7a789056bad95b54dfd3bee73ff/onnxruntime-1.23.2-cp311-cp311-macosx_13_0_arm64.whl",
                "6f91d2c9b0965e86827a5ba01531d5b669770b01775b23199565d6c1f136616c",
                vec![("libonnxruntime.1.23.2.dylib", "c1440b79237783705e156956469f207d51d03d74003886197972af76b966c04c", true)]),
        };
        members.push(("LICENSE", if self.windows() { "c250d6278f0b47a6439fb7592b08b58a55eb9f535aa49a1db63211c3f982b674" } else { license }, false));
        members.push(("ThirdPartyNotices.txt", if self.windows() { "e00f828e0a33de591a355ae6606d2625f5758da7d2c844db7821c9dd3e3647b6" } else { notices }, false));
        Runtime { target: self.name(), version: ORT_VERSION, wheel_url: url, wheel_sha256: wheel,
            files: members.into_iter().map(|(name, sha256, library)| RuntimeFile { name, sha256, library }).collect() }
    }

    pub(crate) fn admit(self, file: &Path, library: bool) -> Result<()> {
        let mut file = files::regular(file)?;
        self.admit_reader(&mut file, library)
    }

    pub(crate) fn admit_program(self, path: &Path, program: &str) -> Result<()> {
        self.admit_program_reader(&mut files::regular(path)?, program)
    }

    pub(crate) fn admit_program_reader(self, file: &mut (impl Read + Seek), program: &str) -> Result<()> {
        file.seek(SeekFrom::Start(0))?;
        self.admit_reader(file, false)?;
        if self.windows() {
            file.seek(SeekFrom::Start(60))?;
            let mut offset = [0u8; 4]; file.read_exact(&mut offset)?;
            file.seek(SeekFrom::Start(u32::from_le_bytes(offset) as u64 + 24 + 68))?;
            let mut subsystem = [0u8; 2]; file.read_exact(&mut subsystem)?;
            let expected = if matches!(program, "verdant-gui" | "verdant-launcher") { 2 } else { 3 };
            ensure!(u16::from_le_bytes(subsystem) == expected, "wrong Windows GUI/console subsystem for {program}");
        }
        Ok(())
    }

    pub(crate) fn admit_reader(self, file: &mut (impl Read + Seek), library: bool) -> Result<()> {
        let mut header = [0u8; 64];
        file.read_exact(&mut header)?;
        match self {
            Self::LinuxX64 | Self::LinuxArm64 => {
                ensure!(header[..7] == [0x7f, b'E', b'L', b'F', 2, 1, 1]
                    && matches!(u16::from_le_bytes([header[16], header[17]]), 2 | 3)
                    && u16::from_le_bytes([header[18], header[19]]) == if self == Self::LinuxX64 { 62 } else { 183 }, "invalid 64-bit Linux native input");
            }
            Self::WindowsX64 => {
                ensure!(&header[..2] == b"MZ", "missing Windows executable header");
                let offset = u32::from_le_bytes(header[60..64].try_into()?);
                ensure!((64..=64 * 1024).contains(&offset), "invalid PE header offset");
                file.seek(SeekFrom::Start(offset as u64))?;
                let mut pe = [0u8; 26]; file.read_exact(&mut pe)?;
                let flags = u16::from_le_bytes([pe[22], pe[23]]);
                ensure!(&pe[..6] == b"PE\0\0\x64\x86" && &pe[24..] == b"\x0b\x02"
                    && flags & 2 != 0 && (flags & 0x2000 != 0) == library, "invalid x64 PE executable/library");
            }
            Self::MacArm64 => {
                ensure!(&header[..8] == b"\xcf\xfa\xed\xfe\x0c\x00\x00\x01"
                    && u32::from_le_bytes(header[12..16].try_into()?) == if library { 6 } else { 2 }, "invalid Apple Silicon Mach-O executable/library");
            }
        }
        Ok(())
    }
}
