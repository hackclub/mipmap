import { message } from "$lib/api";
import { page_size } from "./api";

export class Pages<T extends { id: number }> {
  items = $state.raw<T[]>([]);
  error = $state("");
  loading = $state(false);
  done = $state(false);
  #run = 0;

  constructor(private fetcher: (before?: number) => Promise<T[]>) {}

  reset = () => {
    this.#run++;
    this.items = [];
    this.done = false;
    return this.more();
  };

  more = async () => {
    const run = ++this.#run;
    this.loading = true;
    this.error = "";
    try {
      const items = await this.fetcher(this.items.at(-1)?.id);
      if (run !== this.#run) return;
      this.items = [...this.items, ...items];
      this.done = items.length < page_size;
    } catch (e) {
      if (run === this.#run) this.error = message(e);
    } finally {
      if (run === this.#run) this.loading = false;
    }
  };

  replace = (item: T) => {
    this.items = this.items.map((old) => (old.id === item.id ? item : old));
  };

  remove = (id: number) => {
    this.items = this.items.filter((item) => item.id !== id);
  };
}
