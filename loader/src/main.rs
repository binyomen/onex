use scopeguard::ScopeGuard;
use std::{
    env,
    fs::{self, create_dir_all, remove_dir_all, File},
    io::{self, Read, Seek, SeekFrom},
    path::PathBuf,
    process::Command,
    ptr,
};
use uuid::Uuid;
use winapi::um::{
    wincon::GetConsoleWindow,
    winuser::{ShowWindow, SW_HIDE},
};
use zip::ZipArchive;

struct OffsetSeeker {
    file: File,
    offset: u64,
    cursor: u64,
    length: u64,
}

impl OffsetSeeker {
    fn new(mut file: File, offset: u64, length: u64) -> io::Result<Self> {
        file.seek(SeekFrom::Start(offset))?;
        Ok(OffsetSeeker {
            file,
            offset,
            cursor: 0,
            length,
        })
    }
}

impl Seek for OffsetSeeker {
    fn seek(&mut self, seek_from: SeekFrom) -> io::Result<u64> {
        let new_cursor = match seek_from {
            SeekFrom::Start(i) => i as i64,
            SeekFrom::End(i) => self.length as i64 + i,
            SeekFrom::Current(i) => self.cursor as i64 + i,
        };

        if new_cursor < 0 {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot seek before byte 0.",
            ))
        } else {
            self.cursor = if new_cursor <= self.length as i64 {
                new_cursor as u64
            } else {
                self.length
            };

            self.file.seek(SeekFrom::Start(self.cursor + self.offset))?;
            Ok(self.cursor)
        }
    }
}

impl Read for OffsetSeeker {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let resized_buf = if self.cursor + buf.len() as u64 > self.length {
            &mut buf[..(self.length - self.cursor) as usize]
        } else {
            buf
        };
        let result = self.file.read(resized_buf)?;
        self.cursor += result as u64;
        Ok(result)
    }
}

fn main() {
    real_main().unwrap();
}

fn real_main() -> io::Result<()> {
    // When a packaged app is run from the start menu, a console window is
    // created for the loader. When it's a GUI app, the console window stays
    // open while the GUI stays open. And if you close the console window
    // before closing the app itself, the loader exits and won't clean up after
    // itself. Hide the console window so this isn't an issue.
    hide_console_window();

    let exe_path = env::current_exe()?;
    let mut file = File::open(exe_path)?;

    file.seek(SeekFrom::End(-8))?;
    let mut data_offset = [0; 8];
    file.read_exact(&mut data_offset)?;
    let data_offset = u64::from_le_bytes(data_offset);

    let file_length = file.metadata()?.len();
    let data_length = file_length - data_offset - 8;

    let seeker = OffsetSeeker::new(file, data_offset, data_length)?;
    run_app(seeker)?;

    Ok(())
}

fn hide_console_window() {
    let window = unsafe { GetConsoleWindow() };
    if window != ptr::null_mut() {
        unsafe {
            ShowWindow(window, SW_HIDE);
        }
    }
}

fn run_app(seeker: OffsetSeeker) -> io::Result<()> {
    let mut archive = ZipArchive::new(seeker)?;
    let mut uuid_buffer = Uuid::encode_buffer();
    let instance_id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut uuid_buffer);
    let temp_dir = [env::temp_dir(), PathBuf::from(instance_id.to_owned())]
        .iter()
        .collect();
    let temp_dir = scopeguard::guard(temp_dir, |d| {
        let _ = remove_dir_all(d);
    });

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let output_path: PathBuf = [&temp_dir, &PathBuf::from(entry.name())].iter().collect();

        if entry.is_file() {
            let parent = output_path.parent().unwrap();
            if !parent.exists() {
                create_dir_all(&parent)?;
            }

            let mut output_file = File::create(&output_path)?;
            io::copy(&mut entry, &mut output_file)?;
        } else {
            create_dir_all(&output_path)?;
        }
    }

    run(temp_dir)?;

    Ok(())
}

fn run<F: FnOnce(std::path::PathBuf)>(app_dir: ScopeGuard<PathBuf, F>) -> io::Result<()> {
    let exe_name_file: PathBuf = [&app_dir, &PathBuf::from("sexe_run")].iter().collect();
    let exe_name = fs::read_to_string(exe_name_file)?.trim().to_owned();
    let exe_file: PathBuf = [&app_dir, &PathBuf::from(exe_name)].iter().collect();

    let args: Vec<String> = env::args().skip(1).collect();
    Command::new(exe_file).args(&args).spawn()?.wait()?;
    Ok(())
}
