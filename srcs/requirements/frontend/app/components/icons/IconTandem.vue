<template>
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" :width="size" :height="size" aria-hidden="true">
    <!-- A shared page -->
    <rect x="4" y="3" width="24" height="26" rx="4" fill="#fafaf9" stroke="#a8a29e" stroke-width="1.2" />
    <g stroke="#d6d3d1" stroke-width="1.6" stroke-linecap="round">
      <path d="M9 10 H23" />
      <path d="M9 15 H19" />
      <path d="M9 20 H23" />
      <path d="M9 25 H16" />
    </g>
    <!-- Two writers' carets, each with a name flag, as the editor shows collaborators -->
    <g class="caret caret-0">
      <rect x="13" y="9" width="6" height="3.5" rx="1" fill="#547a5c" />
      <path d="M13 12 V18.5" stroke="#547a5c" stroke-width="1.6" stroke-linecap="round" />
    </g>
    <g class="caret caret-1">
      <rect x="21" y="14" width="6" height="3.5" rx="1" fill="#d97706" />
      <path d="M21 17 V23.5" stroke="#d97706" stroke-width="1.6" stroke-linecap="round" />
    </g>
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ size?: number }>(), { size: 64 });
</script>

<style scoped>
/* the carets blink out of step, like two people typing */
.caret {
  animation: blink 1.6s steps(1) infinite;
}
.caret-1 {
  animation-delay: 0.8s;
}
@keyframes blink {
  50% {
    opacity: 0.35;
  }
}
@media (prefers-reduced-motion: reduce) {
  .caret {
    animation: none;
  }
}
</style>
