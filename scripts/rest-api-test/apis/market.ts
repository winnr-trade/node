import {
  adminAddress,
  adminSigner,
  chainState,
  tokenMinterSigner,
  rollup,
  userSigner,
} from "../config";
import testMarketData from "../../../test-data/market/data.json";
import type { Signer } from "@sovereign-sdk/signers";

export const setSupportedCollateralToken = async (params: {
  tokenId: string;
  support: boolean;
}) => {
  const callMessage = {
    market: {
      set_supported_collateral_token: {
        token_id: params.tokenId,
        support: params.support,
      },
    },
  };

  const res = await rollup.call(callMessage, { signer: adminSigner });
  return res;
};

type ResolverAddress = {
  Address: string;
};

type ResolverPyth = {
  Pyth: {
    feed_id: number[];
    lower_bound?: number;
    upper_bound?: number;
  };
};

type ResolverOptimistic = { Optimistic: {} };

export type Resolver = ResolverAddress | ResolverPyth | ResolverOptimistic;

export const createMarket = async (params: {
  question: string;
  collateralTokenId: string;
  resolutionTime: number;
  resolver: Resolver;
}) => {
  const callMessage = {
    market: {
      create_market: {
        question: params.question,
        collateral_token: params.collateralTokenId,
        resolution_time: params.resolutionTime, // 24 hours from now
        resolver: params.resolver,
      },
    },
  };

  try {
    const res = await rollup.call(callMessage, { signer: tokenMinterSigner });
    return res;
  } catch (error) {
    console.error("Error:\n", JSON.stringify(error));
    throw error;
  }
};

export const createMarkets = async (collateralTokenId: string) => {
  const currentTime = await chainState.time();

  for (const marketData of testMarketData.markets) {
    let resolver: Resolver;
    if (marketData.resolver_type === "address") {
      resolver = { Address: adminAddress };
    } else if (marketData.resolver_type === "pyth") {
      const r = marketData.resolver as {
        Pyth: {
          feed_id: number[];
          lower_bound: number | null;
          upper_bound: number | null;
        };
      };
      resolver = {
        Pyth: {
          feed_id: r.Pyth.feed_id,
          lower_bound: r.Pyth.lower_bound ?? undefined,
          upper_bound: r.Pyth.upper_bound ?? undefined,
        },
      };
    } else if (marketData.resolver_type === "optimistic") {
      resolver = { Optimistic: {} };
    } else {
      throw new Error(`Unknown resolver type: ${marketData.resolver_type}`);
    }

    const callMessage = {
      market: {
        create_market: {
          question: marketData.question,
          collateral_token: collateralTokenId,
          resolution_time: currentTime + 864_000, // 24 hours from now
          resolver,
        },
      },
    };

    const res = await rollup.call(callMessage, { signer: tokenMinterSigner });
  }
};

export const mintShares = async (
  marketId: number,
  amount: number,
  signer: Signer,
) => {
  const callMessage = {
    market: {
      mint_shares: {
        market_id: marketId,
        amount,
      },
    },
  };

  const res = await rollup.call(callMessage, { signer });
};
