use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio_serial::SerialPortBuilderExt;

/// Write text to a serial port asynchronously
///
/// Opens the specified serial port, writes the text with the appropriate suffix,
/// and closes the port. Errors are returned for port access issues.
pub async fn write_to_serial_async(
    text: &str,
    suffix: super::Suffix,
    port_name: &str,
    baud_rate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Open serial port with timeout
    let mut port = tokio_serial::new(port_name, baud_rate)
        .timeout(Duration::from_millis(100))
        .open_native_async()?;

    // Format the output with appropriate suffix
    let output = match suffix {
        super::Suffix::Enter => format!("{}\r\n", text),
        super::Suffix::Tab => format!("{}\t", text),
        super::Suffix::None => text.to_string(),
    };

    // Write to port
    port.write_all(output.as_bytes()).await?;

    // Flush to ensure data is sent
    port.flush().await?;

    Ok(())
}
