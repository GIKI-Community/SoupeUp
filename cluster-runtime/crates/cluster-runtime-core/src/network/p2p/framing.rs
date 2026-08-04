//! Length-prefixed JSON framing over iroh bi-streams.

use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MAX_FRAME: usize = 32 * 1024 * 1024;

pub async fn write_msg<W, T>(writer: &mut W, msg: &T) -> Result<(), String>
where
    W: AsyncWriteExt + Unpin,
    T: serde::Serialize,
{
    let bytes = serde_json::to_vec(msg).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_FRAME {
        return Err(format!("frame too large: {} bytes", bytes.len()));
    }
    let len = (bytes.len() as u32).to_be_bytes();
    writer.write_all(&len).await.map_err(|e| e.to_string())?;
    writer.write_all(&bytes).await.map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn read_msg<R, T>(reader: &mut R) -> Result<T, String>
where
    R: AsyncReadExt + Unpin,
    T: serde::de::DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME {
        return Err(format!("frame too large: {len} bytes"));
    }
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&buf).map_err(|e| e.to_string())
}
