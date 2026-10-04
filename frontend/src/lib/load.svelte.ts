import { message } from "$lib/api";

export class Load<T> {
  data = $state.raw<T>();
  error = $state("");
  #run = 0;

  constructor(private fetcher: () => Promise<T>) {}

  set = (data: T) => {
    this.#run++;
    this.data = data;
    this.error = "";
  };

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

export class Task {
  pending = $state(false);
  error = $state("");

  constructor(private fallback?: string) {}

  run = async (action: () => Promise<unknown>, done?: () => void) => {
    this.pending = true;
    this.error = "";
    try {
      await action();
      done?.();
    } catch (e) {
      this.error = message(e, this.fallback);
    } finally {
      this.pending = false;
    }
  };
}
