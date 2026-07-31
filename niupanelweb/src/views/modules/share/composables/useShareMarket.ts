import { ref } from "vue";
import * as shareApi from "@/api/share";
import type { MarketScriptAggregated } from "@/types";

export function useShareMarket() {
  const marketScripts = ref<MarketScriptAggregated[]>([]);
  const loadingMarket = ref(false);

  const fetchMarketScripts = async () => {
    loadingMarket.value = true;
    try {
      const res = await shareApi.listMarketScripts();
      marketScripts.value = res.data;
    } finally {
      loadingMarket.value = false;
    }
  };

  return {
    fetchMarketScripts,
    loadingMarket,
    marketScripts,
  };
}
