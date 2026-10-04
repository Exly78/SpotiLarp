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
{:else if name === "user"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
    <circle cx="12" cy="8" r="4" />
    <path d="M4.5 20c1.2-3.6 4-5.5 7.5-5.5s6.3 1.9 7.5 5.5" />
  </svg>
{:else if name === "window-minimize"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
    <line x1="6" y1="12" x2="18" y2="12" />
  </svg>
{:else if name === "window-maximize"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
    <rect x="6.5" y="6.5" width="11" height="11" rx="1" />
  </svg>
{:else if name === "window-restore"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round">
    <rect x="6.5" y="9" width="8.5" height="8.5" rx="1" />
    <path d="M9 9V7.5a1 1 0 0 1 1-1h6.5a1 1 0 0 1 1 1V14a1 1 0 0 1-1 1H15" />
  </svg>
{:else if name === "window-close"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
    <line x1="6.5" y1="6.5" x2="17.5" y2="17.5" />
    <line x1="17.5" y1="6.5" x2="6.5" y2="17.5" />
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
{:else if name === "chart"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
    <line x1="6" y1="20" x2="6" y2="13" />
    <line x1="12" y1="20" x2="12" y2="5" />
    <line x1="18" y1="20" x2="18" y2="10" />
  </svg>
{:else if name === "clock"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="9" />
    <polyline points="12,7 12,12 15.5,14" />
  </svg>
{:else if name === "chevron-left"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">
    <polyline points="15,5 8,12 15,19" />
  </svg>
{:else if name === "chevron-right"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">
    <polyline points="9,5 16,12 9,19" />
  </svg>
{:else if name === "settings"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
  </svg>
{:else if name === "mini-player"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <rect x="11" y="12" width="7" height="5" rx="1" fill="currentColor" stroke="none" />
  </svg>
{:else if name === "expand"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <polyline points="14,4 20,4 20,10" />
    <polyline points="10,20 4,20 4,14" />
    <line x1="20" y1="4" x2="13" y2="11" />
    <line x1="4" y1="20" x2="11" y2="13" />
  </svg>
{:else if name === "folder"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2.5h7.5A2.5 2.5 0 0 1 21 10v7.5a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5z" />
  </svg>
{:else if name === "plus"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
    <line x1="12" y1="5" x2="12" y2="19" />
    <line x1="5" y1="12" x2="19" y2="12" />
  </svg>
{:else if name === "moon"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z" />
  </svg>
{:else if name === "shuffle"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M3 7h3.5c2 0 3.2 1 4.3 2.7l2.4 4.6c1.1 1.7 2.3 2.7 4.3 2.7H20" />
    <path d="M3 17h3.5c1.4 0 2.4-0.5 3.2-1.3" />
    <path d="M14.3 8.3c0.8-0.8 1.8-1.3 3.2-1.3H20" />
    <polyline points="17,4 20,7 17,10" />
    <polyline points="17,14 20,17 17,20" />
  </svg>
{:else if name === "queue"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <line x1="3" y1="6" x2="21" y2="6" />
    <line x1="3" y1="12" x2="14" y2="12" />
    <line x1="3" y1="18" x2="11" y2="18" />
    <polygon points="16,15 16,21 21,18" fill="currentColor" stroke="none" />
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
{:else if name === "verified"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24">
    <path
      d="M12 1.5l2.6 1.9 3.2-.1 1 3.1 2.6 1.9-1 3.1 1 3.1-2.6 1.9-1 3.1-3.2-.1L12 22.5l-2.6-1.9-3.2.1-1-3.1-2.6-1.9 1-3.1-1-3.1 2.6-1.9 1-3.1 3.2.1z"
      fill="#4cb3ff"
    />
    <polyline points="7.6,12.2 10.6,15.1 16.4,9.2" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
{:else if name === "add-circle"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round">
    <circle cx="12" cy="12" r="9.5" />
    <line x1="12" y1="7.5" x2="12" y2="16.5" />
    <line x1="7.5" y1="12" x2="16.5" y2="12" />
  </svg>
{:else if name === "added"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24">
    <circle cx="12" cy="12" r="10.3" fill="currentColor" />
    <polyline points="7.5,12.3 10.6,15.3 16.5,9.2" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
{:else if name === "more"}
  <svg {...rest} width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <circle cx="5" cy="12" r="2" />
    <circle cx="12" cy="12" r="2" />
    <circle cx="19" cy="12" r="2" />
  </svg>
{/if}
