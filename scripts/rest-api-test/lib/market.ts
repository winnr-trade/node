// Market class for queries related to the market module

import type { Rollup } from "@sovereign-sdk/web3";

export class Market {
  // biome-ignore lint/suspicious/noExplicitAny: types aren't used
  private readonly rollup: Rollup<any, any>;

  private readonly prefix: string = "/modules/market";

  constructor(rollup: any) {
    this.rollup = rollup;
  }

  async getNextMarketId() {
    const res: any = await this.rollup.http.get(
      `${this.prefix}/state/next-market-id`,
    );
    return res.value as number;
  }

  // Fetch a list of markets, with optional from_id and limit
  async list(from_id: number = 0, limit: number = 10) {
    const query = { from_id, limit };
    return this.rollup.http.get(`${this.prefix}/list`, { query });
  }

  // Fetch a single market by ID
  async get(marketId: number) {
    return this.rollup.http.get(`${this.prefix}/${marketId}`);
  }

  // Fetch market module status
  async status() {
    return this.rollup.http.get(`${this.prefix}/status`);
  }

  // Add more methods as needed (e.g., create, update, etc.)
}
