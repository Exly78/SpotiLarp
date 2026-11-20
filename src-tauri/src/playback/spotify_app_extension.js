// SpotiLarp's link to this Spotify app (written by SpotiLarp: Settings → Spotify desktop app).
// SpotiLarp, running on this computer, tells Spotify what to play here and
// hears what it's doing, so its queue, Jams and listening along can play
// through Spotify, on a free account too. It only ever talks to this computer.
(function spotilarp() {
  const PORT = __SPOTILARP_PORT__;
  const TOKEN = "__SPOTILARP_TOKEN__";
  const Player = window.Spicetify?.Player;
  // Spotify is still starting up.
  if (!Player?.addEventListener || !Player.isPlaying) {
    setTimeout(spotilarp, 300);
    return;
  }

  /** @type {WebSocket|null} */
  let socket = null;
  let retryMs = 2000;
  let lastSent = 0;
  let lastKey = "";

  const imageUrl = (url) => (url?.startsWith("spotify:image:") ? `https://i.scdn.co/image/${url.slice(14)}` : url ?? null);

  // Where the playing song is kept has moved between Spotify versions:
  // Spotify's own player state first (Spicetify's copy can be empty or stale).
  function current() {
    const api = window.Spicetify?.Platform?.PlayerAPI;
    for (const state of [api?._state, api?.getState?.(), Player.data]) {
      const item = state?.item ?? state?.track;
      if (item?.uri) return item;
    }
    return null;
  }

  function report() {
    const item = current();
    const meta = item?.metadata ?? {};
    const uri = item?.uri ?? "";
    const artists = item?.artists?.length
      ? item.artists.map((artist) => ({ name: artist.name, uri: artist.uri }))
      : meta.artist_name
        ? [{ name: meta.artist_name, uri: meta.artist_uri }]
        : [];
    return {
      uri,
      ad: uri.startsWith("spotify:ad:") || item?.type === "ad" || meta.is_advertisement === "true",
      playing: !!Player.isPlaying(),
      position: Player.getProgress() ?? 0,
      duration: Player.getDuration() ?? 0,
      volume: Player.getVolume?.() ?? null,
      name: item?.name ?? meta.title ?? "",
      artists,
      album: item?.album?.name ?? meta.album_title ?? "",
      cover: imageUrl(item?.album?.images?.[0]?.url ?? item?.images?.[0]?.url ?? meta.image_xlarge_url ?? meta.image_large_url ?? meta.image_url),
    };
  }

  // Changes go at once; the rest of the time, a word every few seconds is plenty.
  function send(force) {
    if (socket?.readyState !== WebSocket.OPEN) return;
    const now = Date.now();
    const state = report();
    const key = [state.uri, state.playing, state.ad, Math.round((state.volume ?? 0) * 100)].join("|");
    if (!force && key === lastKey && now - lastSent < 5000) return;
    lastKey = key;
    lastSent = now;
    socket.send(JSON.stringify(state));
  }

  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  async function play(message) {
    if (current()?.uri === message.uri) {
      // Already on it: played again from where it's asked, not restarted (which
      // could land after the seek below).
      if (!message.paused && !Player.isPlaying()) Player.play();
    } else {
      await Player.playUri(message.uri);
      for (let i = 0; i < 50 && current()?.uri !== message.uri; i++) await sleep(100);
    }
    // Spicetify reads 0 to 1 as a fraction of the song.
    if (message.position <= 1) {
      Player.seek(0);
    } else {
      Player.seek(message.position);
      // Reported once it's there, not from where the song started.
      for (let i = 0; i < 20 && Math.abs((Player.getProgress() ?? 0) - message.position) > 1500; i++) await sleep(50);
    }
    if (message.paused) Player.pause();
    send(true);
  }

  function onMessage(event) {
    let message;
    try {
      message = JSON.parse(event.data);
    } catch {
      return;
    }
    switch (message.type) {
      case "play":
        play(message);
        break;
      case "pause":
        if (Player.isPlaying()) Player.pause();
        break;
      case "resume":
        if (!Player.isPlaying()) Player.play();
        break;
      case "seek":
        if (message.position > 1) Player.seek(message.position);
        else Player.seek(0);
        break;
      case "volume":
        Player.setVolume(message.volume);
        break;
    }
  }

  function connect() {
    socket = new WebSocket(`ws://127.0.0.1:${PORT}/?token=${TOKEN}`);
    socket.onopen = () => {
      retryMs = 2000;
      send(true);
    };
    socket.onmessage = onMessage;
    // SpotiLarp isn't running: look again now and then.
    socket.onclose = () => {
      socket = null;
      setTimeout(connect, retryMs);
      retryMs = Math.min(retryMs * 2, 30000);
    };
  }

  Player.addEventListener("songchange", () => send(true));
  Player.addEventListener("onplaypause", () => send(true));
  Player.addEventListener("onprogress", () => send(false));
  connect();
})();
