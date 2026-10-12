use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// A later launch handed its arguments to this running instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launched<'a> {
    /// The arguments, without the program name, in order.
    pub args: Vec<&'a str>,
    /// The launching process's working directory, against which a relative
    /// path among `args` resolves.
    pub cwd: &'a str,
}

impl Launched<'_> {
    /// Encodes the message, or `None` when a field does not fit the wire
    /// format: an argument or the directory over 65535 bytes, or more than
    /// 65535 arguments.
    ///
    /// - The arguments come from whoever launched the application, so their
    ///   size is not the host's to promise.
    /// - [`EncodeMessage::encode`] panics in the same case, matching the
    ///   other messages in this crate whose fields the engine controls.
    pub fn try_encode(&self) -> Option<Vec<u8>> {
        let count: u16 = self.args.len().try_into().ok()?;
        let mut writer = Writer::new().try_str(self.cwd).ok()?.u16(count);
        for arg in &self.args {
            writer = writer.try_str(arg).ok()?;
        }
        Some(writer.finish())
    }
}

impl Message for Launched<'_> {
    const TOPIC: &'static str = "instance/launched";
}

impl EncodeMessage for Launched<'_> {
    fn encode(&self) -> Vec<u8> {
        self.try_encode()
            .expect("launch arguments exceed what instance/launched can frame")
    }
}

impl<'a> DecodeMessage<'a> for Launched<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let cwd = reader.read_str()?;
        let count = reader.read_u16()?;
        let args = (0..count)
            .map(|_| reader.read_str())
            .collect::<Result<Vec<_>, _>>()?;
        reader.finish()?;
        Ok(Self { args, cwd })
    }
}
