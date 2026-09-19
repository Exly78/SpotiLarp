<script>
  let { name, size = 20, ...rest } = $props();
</script>

{#if name === "search"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <circle cx="10" cy="10" r="6" />
    <line x1="20" y1="20" x2="14.5" y2="14.5" />
  </svg>
{:else if name === "play"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24">
    <polygon points="6,4 20,12 6,20" fill="currentColor" />
  </svg>
{:else if name === "pause"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24">
    <rect x="6" y="4" width="4" height="16" rx="1" fill="currentColor" />
    <rect x="14" y="4" width="4" height="16" rx="1" fill="currentColor" />
  </svg>
{:else if name === "previous"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24">
    <rect x="4" y="4" width="2.5" height="16" rx="1" fill="currentColor" />
    <polygon points="18,4 18,20 8,12" fill="currentColor" />
  </svg>
{:else if name === "next"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24">
    <polygon points="6,4 6,20 16,12" fill="currentColor" />
    <rect x="17.5" y="4" width="2.5" height="16" rx="1" fill="currentColor" />
  </svg>
{:else if name === "volume"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <path d="M4 15h4l5 5V4l-5 5H4v6z" fill="currentColor" stroke="none" />
    <path d="M15.5 8.5 Q18.5 12 15.5 15.5" />
    <path d="M18 5.5 Q23 12 18 18.5" />
  </svg>
{:else if name === "volume-mute"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <path d="M4 15h4l5 5V4l-5 5H4v6z" fill="currentColor" stroke="none" />
    <line x1="16" y1="9" x2="21" y2="15" />
    <line x1="21" y1="9" x2="16" y2="15" />
  </svg>
{:else if name === "refresh"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <circle cx="12" cy="12" r="8" stroke-dasharray="37 12" />
  </svg>
{:else if name === "panel-right"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <rect x="15" y="4" width="6" height="16" rx="2" fill="currentColor" stroke="none" />
  </svg>
{:else if name === "close"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <line x1="5" y1="5" x2="19" y2="19" />
    <line x1="19" y1="5" x2="5" y2="19" />
  </svg>
{:else if name === "home"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M4 11.5 12 4l8 7.5" />
    <path d="M6 10v9h12v-9" />
  </svg>
{:else if name === "lyrics"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M5 4h11l3 3v13H5z" />
    <line x1="8" y1="9" x2="15" y2="9" />
    <line x1="8" y1="13" x2="16" y2="13" />
    <line x1="8" y1="17" x2="12" y2="17" />
  </svg>
{:else if name === "heart"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M12 20.5s-7.5-4.6-10-9.3C0.3 7.9 2 4.5 5.4 4c2.1-0.3 4 0.8 6.6 3.4C14.6 4.8 16.5 3.7 18.6 4c3.4 0.5 5.1 3.9 3.4 7.2C19.5 15.9 12 20.5 12 20.5z" />
  </svg>
{:else if name === "heart-filled"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24">
    <path d="M12 20.5s-7.5-4.6-10-9.3C0.3 7.9 2 4.5 5.4 4c2.1-0.3 4 0.8 6.6 3.4C14.6 4.8 16.5 3.7 18.6 4c3.4 0.5 5.1 3.9 3.4 7.2C19.5 15.9 12 20.5 12 20.5z" fill="currentColor" />
  </svg>
{:else if name === "repeat"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M4 7h13a3 3 0 0 1 3 3v2" />
    <polyline points="14,3 17,7 14,11" />
    <path d="M20 17H7a3 3 0 0 1-3-3v-2" />
    <polyline points="10,13 7,17 10,21" />
  </svg>
{:else if name === "discord"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M6.5 6.5c3.5-1.6 7.5-1.6 11 0" />
    <path d="M5 8.2C2.8 12 2.5 15.7 3.4 18.5c1.6 1.4 3.4 2.1 5.1 2.5l1-1.9" />
    <path d="M19 8.2c2.2 3.8 2.5 7.5 1.6 10.3-1.6 1.4-3.4 2.1-5.1 2.5l-1-1.9" />
    <path d="M8.4 15c-3 0-3-4.6 0-4.6s3 4.6 0 4.6z" fill="currentColor" stroke="none" />
    <path d="M15.6 15c-3 0-3-4.6 0-4.6s3 4.6 0 4.6z" fill="currentColor" stroke="none" />
  </svg>
{:else if name === "repeat-one"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M4 7h13a3 3 0 0 1 3 3v2" />
    <polyline points="14,3 17,7 14,11" />
    <path d="M20 17H7a3 3 0 0 1-3-3v-2" />
    <polyline points="10,13 7,17 10,21" />
    <text x="12" y="14.5" font-size="7" font-weight="bold" text-anchor="middle" stroke="none" fill="currentColor">1</text>
  </svg>
{/if}
