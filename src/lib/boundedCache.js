/**
 * A Map that keeps only its `limit` most recently used entries, so caches of
 * pages you've visited don't grow for as long as the app stays open.
 * @template K, V
 * @extends {Map<K, V>}
 */
export class BoundedCache extends Map {
  /** @param {number} limit */
  constructor(limit) {
    super();
    this.limit = limit;
  }

  /** @param {K} key */
  get(key) {
    if (!super.has(key)) return undefined;
    const value = /** @type {V} */ (super.get(key));
    // Used again: move it to the newest end.
    super.delete(key);
    super.set(key, value);
    return value;
  }

  /**
   * @param {K} key
   * @param {V} value
   */
  set(key, value) {
    super.delete(key);
    super.set(key, value);
    while (this.size > this.limit) {
      const oldest = this.keys().next();
      if (oldest.done) break;
      super.delete(oldest.value);
    }
    return this;
  }
}
