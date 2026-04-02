import { Secp256k1Signer } from "@sovereign-sdk/signers";
import { createStandardRollup } from "@sovereign-sdk/web3";
import { Bank } from "@sovereign-sdk/modules";
import adminKey from "../../test-data/keys/admin.json";
import minterKey from "../../test-data/keys/minter_private_key.json";
import signerKey from "../../test-data/keys/tx_signer_private_key.json";
import tokenDeployerKey from "../../test-data/keys/token_deployer_private_key.json";
import { ChainState } from "./lib/chainState";
import { Market } from "./lib/market";

export const ROLLUP_ENDPOINT = "http://localhost:12346";
export const API_BASE_URL = `${ROLLUP_ENDPOINT}/modules`;

export const rollup = await createStandardRollup({ url: ROLLUP_ENDPOINT });
export const bank = new Bank(rollup);
export const market = new Market(rollup);
export const chainState = new ChainState(rollup);

export const minterSigner = new Secp256k1Signer(minterKey.private_key);
export const adminSigner = new Secp256k1Signer(adminKey.private_key);
export const signer = new Secp256k1Signer(signerKey.private_key);
export const tokenDeployerSigner = new Secp256k1Signer(
  tokenDeployerKey.private_key,
);
export const minterAddress = minterKey.address;
export const adminAddress = adminKey.address;
export const signerAddress = signerKey.address;
export const tokenDeployerAddress = tokenDeployerKey.address;

export const routes = {
  bank: {
    getToken: (tokenId: string) => `${API_BASE_URL}/bank/tokens/${tokenId}`,
    balance: (account: string) => `${API_BASE_URL}/bank/balance/${account}`,
  },
  market: {
    status: `${API_BASE_URL}/market/status`,
    list: (fromId: number) => `${API_BASE_URL}/market/list/${fromId}`,
    details: (marketId: string) => `${API_BASE_URL}/market/${marketId}`,
  },
};
