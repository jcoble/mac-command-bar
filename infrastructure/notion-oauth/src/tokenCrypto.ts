const encoder = new TextEncoder();
const decoder = new TextDecoder();

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function base64ToBytes(value: string): Uint8Array {
  const binary = atob(value);
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

async function encryptionKey(encodedKey: string): Promise<CryptoKey> {
  const bytes = base64ToBytes(encodedKey);
  if (bytes.byteLength !== 32) throw new Error('TOKEN_ENCRYPTION_KEY must contain 32 bytes');
  return await crypto.subtle.importKey('raw', bytes, 'AES-GCM', false, ['encrypt', 'decrypt']);
}

export async function encryptToken(token: string, encodedKey: string): Promise<string> {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const encrypted = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    await encryptionKey(encodedKey),
    encoder.encode(token)
  );
  return `${bytesToBase64(iv)}.${bytesToBase64(new Uint8Array(encrypted))}`;
}

export async function decryptToken(payload: string, encodedKey: string): Promise<string> {
  const [encodedIv, encodedCiphertext] = payload.split('.', 2);
  if (!encodedIv || !encodedCiphertext) throw new Error('Invalid encrypted token');
  const decrypted = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: base64ToBytes(encodedIv) },
    await encryptionKey(encodedKey),
    base64ToBytes(encodedCiphertext)
  );
  return decoder.decode(decrypted);
}
