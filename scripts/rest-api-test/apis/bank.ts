import { getTokenId } from "@sovereign-sdk/modules";
import {
  bank,
  rollup,
  userAddress,
  tokenDeployerAddress,
  tokenDeployerSigner,
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

export const createToken = async () => {
  const tokenIdBytes = getTokenId(
    b58.decode(tokenDeployerAddress),
    testUsd.name,
    testUsd.decimals,
  );
  const tokenId = bech32m.encode("token_", bech32m.toWords(tokenIdBytes));
  let token = await getTokenMetadata(tokenId);

  if (token) {
    // console.log("Token already exists:", token);
  } else {
    const callMessage = {
      bank: {
        create_token: {
          token_name: testUsd.name,
          token_decimals: testUsd.decimals,
          initial_balance: Number(testUsd.initialBalance),
          supply_cap: Number(testUsd.supplyCap),
          mint_to_address: userAddress,
          admins: [],
        },
      },
    };

    const res = await rollup.call(callMessage, { signer: tokenDeployerSigner });

    token = await bank.tokenMetadata(tokenId);
  }

  return { id: tokenId, ...token };
};
