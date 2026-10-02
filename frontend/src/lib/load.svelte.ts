import { message } from "$lib/api";

export class Load<T> {
  data = $state.raw<T>();
  error = $state("");
  #run = 0;

  constructor(private fetcher: () => Promise<T>) {}

  run = async () => {
    const run = ++this.#run;
    this.error = "";
    try {
      const data = await this.fetcher();
      if (run === this.#run) this.data = data;
    } catch (e) {
      if (run === this.#run) this.error = message(e);
    }
  };
}
