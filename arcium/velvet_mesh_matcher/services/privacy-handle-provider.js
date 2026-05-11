const http = require('http');
const { webcrypto } = require('crypto');
const anchor = require('@coral-xyz/anchor');
const { PublicKey } = require('@solana/web3.js');
const {
  getMXEPublicKey,
  RescueCipher,
  x25519,
} = require('@arcium-hq/client');

const PORT = Number(process.env.PORT || 8787);
const RPC_URL = process.env.ANCHOR_PROVIDER_URL
  || process.env.NEXT_PUBLIC_HELIUS_RPC_URL
  || 'https://devnet.helius-rpc.com/?api-key=2d8978c6-7067-459f-ae97-7ea035f1a0cb';
const MATCHER_PROGRAM_ID = new PublicKey(
  process.env.VELVET_MATCHER_PROGRAM_ID || 'CEjM2iFeNzKwDtc8uGLAGVFDoaHvJmy9EunRUwAsJH8e'
);
const SIGN_PDA_SEED = Buffer.from('ArciumSignerAccount');

function json(res, status, body) {
  res.writeHead(status, {
    'content-type': 'application/json',
    'access-control-allow-origin': process.env.CORS_ORIGIN || '*',
    'access-control-allow-methods': 'GET,POST,OPTIONS',
    'access-control-allow-headers': 'content-type,authorization',
  });
  res.end(JSON.stringify(body));
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let raw = '';
    req.on('data', (chunk) => {
      raw += chunk;
      if (raw.length > 16_384) {
        reject(new Error('Request body too large.'));
        req.destroy();
      }
    });
    req.on('end', () => {
      try {
        resolve(raw ? JSON.parse(raw) : {});
      } catch {
        reject(new Error('Invalid JSON body.'));
      }
    });
    req.on('error', reject);
  });
}

function parsePositiveAmount(value) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error('amount must be a positive number.');
  }
  return parsed;
}

function toU64(value) {
  const rounded = Math.max(1, Math.floor(value));
  return BigInt(rounded);
}

function toU16(value) {
  const rounded = Math.max(0, Math.min(65535, Math.floor(value)));
  return BigInt(rounded);
}

function toU8(value) {
  const rounded = Math.max(0, Math.min(255, Math.floor(value)));
  return BigInt(rounded);
}

function makeProvider() {
  const wallet = {
    publicKey: PublicKey.default,
    signTransaction: async () => {
      throw new Error('privacy provider is read-only');
    },
    signAllTransactions: async () => {
      throw new Error('privacy provider is read-only');
    },
  };
  const connection = new anchor.web3.Connection(RPC_URL, 'confirmed');
  return new anchor.AnchorProvider(connection, wallet, {
    commitment: 'confirmed',
    preflightCommitment: 'confirmed',
  });
}

async function encryptIntentTerms(body) {
  const amount = parsePositiveAmount(body.amount);
  const amountDecimals = body.inputSymbol === 'USDC' ? 6 : 9;
  const sizeAtoms = amount * 10 ** amountDecimals;
  const limitPriceBps = Number(body.limitPriceBps || 10_000);
  const slippageBps = Number(body.slippageBps || 50);
  const riskPreference = Number(body.riskPreference || 1);

  const provider = makeProvider();
  const mxePublicKey = await getMXEPublicKey(provider, MATCHER_PROGRAM_ID);
  if (!mxePublicKey) {
    throw new Error('Arcium MXE public key is unavailable for the matcher program.');
  }

  const privateKey = x25519.utils.randomSecretKey();
  const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
  const cipher = new RescueCipher(sharedSecret);
  const nonce = Buffer.from(webcrypto.getRandomValues(new Uint8Array(16)));
  const ciphertexts = cipher.encrypt([
    toU64(sizeAtoms),
    toU16(limitPriceBps),
    toU16(slippageBps),
    toU8(riskPreference),
  ], nonce);
  const [matchVerifier] = PublicKey.findProgramAddressSync([SIGN_PDA_SEED], MATCHER_PROGRAM_ID);

  return {
    encryptedSize: ciphertexts[0],
    encryptedLimitPrice: ciphertexts[1],
    encryptedSlippageBps: ciphertexts[2],
    encryptedRiskPreference: ciphertexts[3],
    matchVerifier: matchVerifier.toBase58(),
    settlementVerifier: body.owner,
    provider: 'arcium-devnet-mxe',
    matcherProgramId: MATCHER_PROGRAM_ID.toBase58(),
    encryptionPublicKey: Array.from(x25519.getPublicKey(privateKey)),
    nonce: Array.from(nonce),
  };
}

async function handler(req, res) {
  if (req.method === 'OPTIONS') {
    return json(res, 204, {});
  }

  if (req.method === 'GET' && req.url === '/health') {
    return json(res, 200, {
      ok: true,
      rpcUrl: RPC_URL.replace(/api-key=[^&]+/, 'api-key=redacted'),
      matcherProgramId: MATCHER_PROGRAM_ID.toBase58(),
    });
  }

  if (req.method === 'POST' && req.url === '/privacy-handles') {
    try {
      const body = await readBody(req);
      if (!body.owner) {
        throw new Error('owner is required.');
      }
      new PublicKey(body.owner);
      const handles = await encryptIntentTerms(body);
      return json(res, 200, handles);
    } catch (error) {
      return json(res, 400, {
        error: error.message || 'Unable to create privacy handles.',
      });
    }
  }

  return json(res, 404, { error: 'Not found.' });
}

http.createServer(handler).listen(PORT, () => {
  console.log(`VelvetMesh privacy handle provider listening on :${PORT}`);
});
