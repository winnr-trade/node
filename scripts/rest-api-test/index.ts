import { market, rollup } from "./config";
import { createToken } from "./apis/bank";
import {
  createMarkets,
  mintShares,
  setSupportedCollateralToken,
} from "./apis/market";
import { placeOrder } from "./apis/orderbook";

const main = async () => {
  console.log("Rollup context:", rollup.context);
  console.log("Creating token...");

  const token = await createToken();
  console.log("Token:", token);

  console.log("Setting token as supported collateral...");
  await setSupportedCollateralToken(token.id, true);

  console.log("Creating market...");
  const marketCount = await market.getNextMarketId();
  if (marketCount === 0) {
    await createMarkets(token.id);
  }

  console.log("Providing liquidity to market...");
  const marketId = 0;
  await mintShares(marketId, 10000);
  // await placeOrder(0, "yes", 50, 100, "ask", "limit");
  // await placeOrder(0, "no", 60, 100, "ask", "limit");

  // Place asks (sellers) at various price levels
  for (let price = 60; price <= 100; price += 10) {
    await placeOrder(marketId, "yes", price, 100, "ask", "limit");
    await placeOrder(marketId, "no", price, 100, "ask", "limit");
  }

  // Place bids (buyers) at various price levels
  for (let price = 40; price <= 80; price += 10) {
    await placeOrder(marketId, "yes", price, 100, "bid", "limit");
    await placeOrder(marketId, "no", price, 100, "bid", "limit");
  }

  // Optionally, place some market orders to create fills
  await placeOrder(marketId, "yes", 100, 50, "bid", "market");
  await placeOrder(marketId, "no", 100, 50, "bid", "market");
};

main()
  .then(() => {
    console.log("Done!");
  })
  .catch((err) => {
    console.error("Error:");
    console.error(JSON.stringify(err, null, 2));
  });
