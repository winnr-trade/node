import type { Signer } from "@sovereign-sdk/signers";
import { rollup, userSigner } from "../config";

export const placeOrder = async (
  params: {
    marketId: number;
    outcome: "yes" | "no";
    price: number;
    quantity: number;
    side: "bid" | "ask";
    orderType:
      | "limit"
      | "market"
      | "post_only"
      | "immediate_or_cancel"
      | "fill_or_kill";
  },
  signer: Signer,
) => {
  const callMessage = {
    orderbook: {
      place_order_normal: {
        market_id: params.marketId,
        outcome: params.outcome,
        side: params.side,
        price: params.price,
        quantity: params.quantity,
        order_type: params.orderType,
      },
    },
  };

  const res = await rollup.call(callMessage, { signer });
  return res;
};
