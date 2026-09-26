use anyhow::{anyhow, Result};
use snow::{Builder, HandshakeState, TransportState};

pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// Noise Protocol XX Initiator state
pub struct NovaNoiseInitiator {
    state: HandshakeState,
}

/// Noise Protocol XX Responder state
pub struct NovaNoiseResponder {
    state: HandshakeState,
}

/// Encrypted transport session established after Noise handshake
pub struct NovaNoiseTransport {
    state: TransportState,
}

impl NovaNoiseInitiator {
    /// Initialize initiator with local 32-byte static X25519 private key.
    pub fn new(local_static_key: &[u8; 32]) -> Result<Self> {
        let pattern = NOISE_PATTERN.parse()
            .map_err(|e| anyhow!("Failed to parse Noise pattern: {:?}", e))?;
        let state = Builder::new(pattern)
            .local_private_key(local_static_key)
            .build_initiator()
            .map_err(|e| anyhow!("Failed to build Noise initiator: {:?}", e))?;
        Ok(Self { state })
    }

    /// Step 1: Initiator writes message 1 -> -> e
    pub fn write_message_1(&mut self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; 1024];
        let len = self.state.write_message(&[], &mut buf)
            .map_err(|e| anyhow!("Noise write_message_1 error: {:?}", e))?;
        buf.truncate(len);
        Ok(buf)
    }

    /// Step 2: Initiator reads message 2 <- <- e, ee, s, es
    pub fn read_message_2(&mut self, msg2: &[u8]) -> Result<()> {
        let mut buf = vec![0u8; 1024];
        self.state.read_message(msg2, &mut buf)
            .map_err(|e| anyhow!("Noise read_message_2 error: {:?}", e))?;
        Ok(())
    }

    /// Step 3: Initiator writes message 3 -> -> s, se and transitions to transport state.
    pub fn write_message_3(mut self) -> Result<(Vec<u8>, NovaNoiseTransport)> {
        let mut buf = vec![0u8; 1024];
        let len = self.state.write_message(&[], &mut buf)
            .map_err(|e| anyhow!("Noise write_message_3 error: {:?}", e))?;
        buf.truncate(len);
        let transport = self.state.into_transport_mode()
            .map_err(|e| anyhow!("Noise into_transport_mode error: {:?}", e))?;
        Ok((buf, NovaNoiseTransport { state: transport }))
    }

    /// Get remote party's static public key once established in Step 2.
    pub fn remote_static_pubkey(&self) -> Option<Vec<u8>> {
        self.state.get_remote_static().map(|k| k.to_vec())
    }
}

impl NovaNoiseResponder {
    /// Initialize responder with local 32-byte static X25519 private key.
    pub fn new(local_static_key: &[u8; 32]) -> Result<Self> {
        let pattern = NOISE_PATTERN.parse()
            .map_err(|e| anyhow!("Failed to parse Noise pattern: {:?}", e))?;
        let state = Builder::new(pattern)
            .local_private_key(local_static_key)
            .build_responder()
            .map_err(|e| anyhow!("Failed to build Noise responder: {:?}", e))?;
        Ok(Self { state })
    }

    /// Step 1: Responder reads message 1
    pub fn read_message_1(&mut self, msg1: &[u8]) -> Result<()> {
        let mut buf = vec![0u8; 1024];
        self.state.read_message(msg1, &mut buf)
            .map_err(|e| anyhow!("Noise responder read_message_1 error: {:?}", e))?;
        Ok(())
    }

    /// Step 2: Responder writes message 2
    pub fn write_message_2(&mut self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; 1024];
        let len = self.state.write_message(&[], &mut buf)
            .map_err(|e| anyhow!("Noise responder write_message_2 error: {:?}", e))?;
        buf.truncate(len);
        Ok(buf)
    }

    /// Step 3: Responder reads message 3 and transitions to transport mode
    pub fn read_message_3(mut self, msg3: &[u8]) -> Result<NovaNoiseTransport> {
        let mut buf = vec![0u8; 1024];
        self.state.read_message(msg3, &mut buf)
            .map_err(|e| anyhow!("Noise responder read_message_3 error: {:?}", e))?;
        let transport = self.state.into_transport_mode()
            .map_err(|e| anyhow!("Noise into_transport_mode error: {:?}", e))?;
        Ok(NovaNoiseTransport { state: transport })
    }

    /// Get remote party's static public key once established in Step 3.
    pub fn remote_static_pubkey(&self) -> Option<Vec<u8>> {
        self.state.get_remote_static().map(|k| k.to_vec())
    }
}

impl NovaNoiseTransport {
    /// Encrypt plaintext with ChaChaPoly AEAD using Noise session state.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; plaintext.len() + 16];
        let len = self.state.write_message(plaintext, &mut buf)
            .map_err(|e| anyhow!("Noise encrypt error: {:?}", e))?;
        buf.truncate(len);
        Ok(buf)
    }

    /// Decrypt ciphertext with ChaChaPoly AEAD using Noise session state.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; ciphertext.len()];
        let len = self.state.read_message(ciphertext, &mut buf)
            .map_err(|e| anyhow!("Noise decrypt error: {:?}", e))?;
        buf.truncate(len);
        Ok(buf)
    }

    /// Check if transport state is still valid.
    pub fn is_initiator(&self) -> bool {
        self.state.is_initiator()
    }
}

/// Generates a fresh static private key for Noise (X25519).
pub fn generate_noise_key() -> Result<[u8; 32]> {
    let builder = Builder::new(NOISE_PATTERN.parse()
        .map_err(|e| anyhow!("Noise pattern parse error: {:?}", e))?);
    let keypair = builder.generate_keypair()
        .map_err(|e| anyhow!("Noise keypair generation error: {:?}", e))?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&keypair.private[..32]);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_xx_handshake_and_transport() -> Result<()> {
        let initiator_key = generate_noise_key()?;
        let responder_key = generate_noise_key()?;

        let mut initiator = NovaNoiseInitiator::new(&initiator_key)?;
        let mut responder = NovaNoiseResponder::new(&responder_key)?;

        // Message 1
        let msg1 = initiator.write_message_1()?;
        responder.read_message_1(&msg1)?;

        // Message 2
        let msg2 = responder.write_message_2()?;
        initiator.read_message_2(&msg2)?;

        // Remote key verification
        assert!(initiator.remote_static_pubkey().is_some());

        // Message 3
        let (msg3, mut initiator_transport) = initiator.write_message_3()?;
        let mut responder_transport = responder.read_message_3(&msg3)?;

        // Transport encryption test: Initiator -> Responder
        let payload = b"Direct encrypted P2P Nova frame";
        let ciphertext = initiator_transport.encrypt(payload)?;
        let decrypted = responder_transport.decrypt(&ciphertext)?;
        assert_eq!(decrypted, payload);

        // Transport encryption test: Responder -> Initiator
        let reply = b"Acknowledged secure connection established";
        let reply_ciphertext = responder_transport.encrypt(reply)?;
        let reply_decrypted = initiator_transport.decrypt(&reply_ciphertext)?;
        assert_eq!(reply_decrypted, reply);

        Ok(())
    }
}
