import { rollup, userSigner } from "../config";

export const placeOrder = async (
  marketId: number,
  outcome: "yes" | "no",
  price: number,
  quantity: number,
  side: "bid" | "ask",
  orderType:
    | "limit"
    | "market"
    | "post_only"
    | "immediate_or_cancel"
    | "fill_or_kill",
) => {
  const callMessage = {
    orderbook: {
      place_order_normal: {
        market_id: marketId,
        outcome: outcome,
        side: side,
        price,
        quantity,
        order_type: orderType,
      },
    },
  };

  const res = await rollup.call(callMessage, { signer: userSigner });
  return res;
};
