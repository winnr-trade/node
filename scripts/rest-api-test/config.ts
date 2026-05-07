import { Ed25519Signer } from "@sovereign-sdk/signers";
import { createStandardRollup } from "@sovereign-sdk/web3";
import { Bank } from "@sovereign-sdk/modules";
import adminKey from "../../test-data/keys/admin.json";
import tokenMinterKey from "../../test-data/keys/token_minter.json";
import userKeys from "../../test-data/keys/users.json";
import tokenDeployerKey from "../../test-data/keys/token_deployer.json";
import marketMakerKey from "../../test-data/keys/market_maker.json";
import { ChainState } from "./lib/chainState";
import { Market } from "./lib/market";

export const ROLLUP_ENDPOINT = "http://localhost:12346";
export const API_BASE_URL = `${ROLLUP_ENDPOINT}/modules`;

export const rollup = await createStandardRollup({ url: ROLLUP_ENDPOINT });
export const bank = new Bank(rollup);
export const market = new Market(rollup);
export const chainState = new ChainState(rollup);

export const tokenMinterSigner = new Ed25519Signer(tokenMinterKey.private_key);
export const adminSigner = new Ed25519Signer(adminKey.private_key);
export const marketMakerSigner = new Ed25519Signer(marketMakerKey.private_key);
export const tokenDeployerSigner = new Ed25519Signer(
  tokenDeployerKey.private_key,
);
export const userSigners = userKeys.data.map(
  (key) => new Ed25519Signer(key.private_key),
);

export const tokenMinterAddress = tokenMinterKey.address;
export const adminAddress = adminKey.address;
export const marketMakerAddress = marketMakerKey.address;
export const tokenDeployerAddress = tokenDeployerKey.address;
export const userAddresses = userKeys.data.map((key) => key.address);

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
