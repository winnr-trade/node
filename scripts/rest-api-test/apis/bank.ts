import { getTokenId } from "@sovereign-sdk/modules";
import {
  bank,
  rollup,
  signerAddress,
  tokenDeployerAddress,
  tokenDeployerSigner,
} from "../config";
import { hexToBytes } from "@sovereign-sdk/utils";
import { bech32m } from "bech32";
import { testUsd } from "../../../test-data/token/data.json";

export const getTokenMetadata = async (tokenId: string) => {
  try {
    const token = await bank.tokenMetadata(tokenId);
    return token;
  } catch (error) {
    // console.log("error", error);
    return null;
  }
};

export const createToken = async () => {
  const tokenIdBytes = getTokenId(
    hexToBytes(tokenDeployerAddress),
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
          mint_to_address: signerAddress,
          admins: [],
        },
      },
    };
    const res = await rollup.call(callMessage, { signer: tokenDeployerSigner });
    token = await bank.tokenMetadata(tokenId);
  }

  return { id: tokenId, ...token };
};
