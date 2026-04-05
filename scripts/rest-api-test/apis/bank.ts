import { getTokenId as _getTokenId } from "@sovereign-sdk/modules";
import {
  bank,
  rollup,
  userAddress,
  tokenDeployerAddress,
  tokenDeployerSigner,
  tokenMinterAddress,
} from "../config";
import { bech32m } from "bech32";
import b58 from "bs58";
import { testUsd } from "../../../test-data/token/data.json";

export const getTokenMetadata = async (tokenId: string) => {
  try {
    const token = await bank.tokenMetadata(tokenId);
    return token;
  } catch (error: any) {
    if (error.status === 404) {
      return null; // Token not found, return null
    }
    throw error; // Rethrow other errors
  }
};

export const getTokenId = (params: {
  deployer: string;
  name: string;
  decimals: number;
}) => {
  const tokenIdBytes = _getTokenId(
    b58.decode(params.deployer),
    params.name,
    params.decimals,
  );
  const tokenId = bech32m.encode("token_", bech32m.toWords(tokenIdBytes));
  return tokenId;
};

export const createToken = async (params: {
  name: string;
  decimals: number;
  initialBalance: number;
  supplyCap: number;
}) => {
  const createMessage = {
    bank: {
      create_token: {
        token_name: params.name,
        token_decimals: params.decimals,
        initial_balance: params.initialBalance,
        supply_cap: params.supplyCap,
        mint_to_address: userAddress,
        admins: [tokenMinterAddress],
      },
    },
  };

  const res = await rollup.call(createMessage, {
    signer: tokenDeployerSigner,
  });

  const tokenId = getTokenId({
    deployer: tokenDeployerAddress,
    name: testUsd.name,
    decimals: testUsd.decimals,
  });
  const token = await bank.tokenMetadata(tokenId);

  return token;
};

export const transferTokens = async (
  tokenId: string,
  toAddress: string,
  amount: number,
) => {
  const transferMessage = {
    bank: {
      transfer: {
        to: toAddress,
        coins: {
          amount: amount,
          token_id: tokenId,
        },
      },
    },
  };

  await rollup.call(transferMessage, {
    signer: tokenDeployerSigner,
  });
};
