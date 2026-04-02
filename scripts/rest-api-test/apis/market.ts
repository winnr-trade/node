import {
  adminAddress,
  adminSigner,
  chainState,
  tokenMinterSigner,
  rollup,
  userSigner,
} from "../config";
import testMarketData from "../../../test-data/market/data.json";

export const setSupportedCollateralToken = async (
  tokenId: string,
  support: boolean,
) => {
  const callMessage = {
    market: {
      set_supported_collateral_token: {
        token_id: tokenId,
        support,
      },
    },
  };

  const res = await rollup.call(callMessage, { signer: adminSigner });
  return res;
};

export const createMarket = async (collateralTokenId: string) => {
  const currentTime = await chainState.time();

  const callMessage = {
    market: {
      create_market: {
        question: "Will it rain tomorrow?",
        collateral_token: collateralTokenId,
        resolution_time: currentTime + 864_000, // 24 hours from now
        resolver: adminAddress,
      },
    },
  };

  const res = await rollup.call(callMessage, { signer: tokenMinterSigner });
  return res;
};

export const createMarkets = async (collateralTokenId: string) => {
  const currentTime = await chainState.time();

  for (const marketData of testMarketData.markets) {
    const callMessage = {
      market: {
        create_market: {
          question: marketData.question,
          collateral_token: collateralTokenId,
          resolution_time: currentTime + 864_000, // 24 hours from now
          resolver: adminAddress,
        },
      },
    };

    const res = await rollup.call(callMessage, { signer: tokenMinterSigner });
    // console.log(`Created market with question: "${marketData.question}"`);
  }
};

export const mintShares = async (marketId: number, amount: number) => {
  const callMessage = {
    market: {
      mint_shares: {
        market_id: marketId,
        amount,
      },
    },
  };

  const res = await rollup.call(callMessage, { signer: userSigner });
};
