use crate::StateId;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC: &[u8; 4] = b"VASF";
const FORMAT_VERSION: u16 = 1;
const ZSTD_LEVEL: i32 = 3;

pub struct VasfHeader {
    pub version: u16,
    pub frame_count: u32,
    pub created_at: i64,
}

pub struct VasfFrame {
    pub state_id: StateId,
    pub timestamp: i64,
    pub vasp_data: Vec<u8>,
    pub delta_data: Option<Vec<u8>>,
}

pub struct VasfFile {
    pub header: VasfHeader,
    pub frames: Vec<VasfFrame>,
}

impl VasfFile {
    pub fn new(frames: Vec<VasfFrame>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Self {
            header: VasfHeader {
                version: FORMAT_VERSION,
                frame_count: frames.len() as u32,
                created_at: now,
            },
            frames,
        }
    }

    pub fn write_to(&self, path: &Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut w = BufWriter::new(file);
        write_header(&mut w, self.frames.len() as u32, self.header.created_at)?;
        for frame in &self.frames {
            write_frame(&mut w, frame)?;
        }
        w.flush()
    }

    pub fn read_from(path: &Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut r = BufReader::new(file);
        let header = read_header(&mut r)?;
        let frames = (0..header.frame_count)
            .map(|_| read_frame(&mut r))
            .collect::<std::io::Result<Vec<_>>>()?;
        Ok(Self { header, frames })
    }
}

fn compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    zstd::encode_all(data, ZSTD_LEVEL)
}

fn decompress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    zstd::decode_all(data)
}

fn write_header<W: Write>(w: &mut W, frame_count: u32, created_at: i64) -> std::io::Result<()> {
    w.write_all(MAGIC)?;
    w.write_all(&FORMAT_VERSION.to_le_bytes())?;
    w.write_all(&frame_count.to_le_bytes())?;
    w.write_all(&created_at.to_le_bytes())?;
    Ok(())
}

fn write_frame<W: Write>(w: &mut W, frame: &VasfFrame) -> std::io::Result<()> {
    w.write_all(&frame.state_id.to_bits().to_le_bytes())?;
    w.write_all(&frame.timestamp.to_le_bytes())?;
    let vasp_c = compress(&frame.vasp_data)?;
    w.write_all(&(vasp_c.len() as u32).to_le_bytes())?;
    w.write_all(&vasp_c)?;
    match &frame.delta_data {
        Some(delta) => {
            let delta_c = compress(delta)?;
            w.write_all(&(delta_c.len() as u32).to_le_bytes())?;
            w.write_all(&delta_c)?;
        }
        None => w.write_all(&0u32.to_le_bytes())?,
    }
    Ok(())
}

fn read_u16_le<R: Read>(r: &mut R) -> std::io::Result<u16> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}

fn read_u32_le<R: Read>(r: &mut R) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_i64_le<R: Read>(r: &mut R) -> std::io::Result<i64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(i64::from_le_bytes(b))
}

fn read_bytes_n<R: Read>(r: &mut R, n: u32) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; n as usize];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

fn read_header<R: Read>(r: &mut R) -> std::io::Result<VasfHeader> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid VASF magic bytes",
        ));
    }
    Ok(VasfHeader {
        version: read_u16_le(r)?,
        frame_count: read_u32_le(r)?,
        created_at: read_i64_le(r)?,
    })
}

fn read_frame<R: Read>(r: &mut R) -> std::io::Result<VasfFrame> {
    let mut bits_b = [0u8; 8];
    r.read_exact(&mut bits_b)?;
    let state_id = StateId::from_bits(u64::from_le_bytes(bits_b));
    let timestamp = read_i64_le(r)?;
    let vasp_len = read_u32_le(r)?;
    let vasp_data = decompress(&read_bytes_n(r, vasp_len)?)?;
    let delta_len = read_u32_le(r)?;
    let delta_data = if delta_len > 0 {
        Some(decompress(&read_bytes_n(r, delta_len)?)?)
    } else {
        None
    };
    Ok(VasfFrame {
        state_id,
        timestamp,
        vasp_data,
        delta_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let state_id = StateId::from_bits(0x0102030405060708);
        let frame = VasfFrame {
            state_id,
            timestamp: 1_700_000_000_000,
            vasp_data: b"=== farscry visual context ===\nscreen_type: config\n".to_vec(),
            delta_data: Some(b"=== farscry diff ===\n".to_vec()),
        };
        let vasf = VasfFile::new(vec![frame]);
        let path = std::path::PathBuf::from("/tmp/_test_vasf_round_trip.vasf");
        vasf.write_to(&path).unwrap();
        let loaded = VasfFile::read_from(&path).unwrap();
        assert_eq!(loaded.frames.len(), 1);
        assert_eq!(loaded.frames[0].state_id.to_bits(), 0x0102030405060708);
        assert_eq!(
            loaded.frames[0].vasp_data,
            b"=== farscry visual context ===\nscreen_type: config\n"
        );
        assert_eq!(
            loaded.frames[0].delta_data,
            Some(b"=== farscry diff ===\n".to_vec())
        );
        assert_eq!(loaded.header.version, 1);
        assert_eq!(loaded.header.frame_count, 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_no_delta() {
        let frame = VasfFrame {
            state_id: StateId::from_bits(0),
            timestamp: 0,
            vasp_data: b"vasp".to_vec(),
            delta_data: None,
        };
        let vasf = VasfFile::new(vec![frame]);
        let path = std::path::PathBuf::from("/tmp/_test_vasf_no_delta.vasf");
        vasf.write_to(&path).unwrap();
        let loaded = VasfFile::read_from(&path).unwrap();
        assert!(loaded.frames[0].delta_data.is_none());
        let _ = std::fs::remove_file(&path);
    }
}
