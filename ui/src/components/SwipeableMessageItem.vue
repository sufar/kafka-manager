<template>
  <div class="swipe-wrapper relative overflow-hidden" @touchstart="onTouchStart" @touchmove="onTouchMove" @touchend="onTouchEnd" @mousedown="onMouseDown">
    <!-- 左侧操作面板（右滑显示） -->
    <div class="absolute top-0 bottom-0 left-0 flex" :style="{ width: leftWidth + 'px' }">
      <slot name="left"></slot>
    </div>

    <!-- 右侧操作面板（左滑显示） -->
    <div class="absolute top-0 bottom-0 right-0 flex justify-end" :style="{ width: rightWidth + 'px' }">
      <slot name="right"></slot>
    </div>

    <!-- 内容层（可滑动） -->
    <div class="content-layer relative z-[1] overflow-hidden transition-all" :class="{ 'no-transition': !animating }" :style="{ transform: 'translateX(' + offset + 'px)' }">
      <slot name="content"></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onUnmounted } from 'vue';

const leftWidth = 60;
const rightWidth = 60;
const offset = ref(0);
let startClientX = 0;
let currentOffset = 0;
let animating = true;

function getClientX(e: TouchEvent | MouseEvent): number {
  return 'touches' in e ? e.touches[0]?.clientX ?? 0 : e.clientX;
}

function onTouchStart(e: TouchEvent) {
  if (e.touches.length !== 1) return;
  startClientX = getClientX(e);
  animating = false;
}

function onTouchMove(e: TouchEvent) {
  if (e.touches.length !== 1) return;
  const dx = getClientX(e) - startClientX;
  let newPos = currentOffset + dx;
  newPos = Math.max(-rightWidth, Math.min(leftWidth, newPos));
  offset.value = newPos;
  startClientX = getClientX(e);
}

function onTouchEnd() {
  snap();
}

function onMouseDown(e: MouseEvent) {
  if (e.button !== 0) return;
  startClientX = e.clientX;
  currentOffset = offset.value;
  animating = false;
  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
}

function onMouseMove(e: MouseEvent) {
  const dx = e.clientX - startClientX;
  let newPos = currentOffset + dx;
  newPos = Math.max(-rightWidth, Math.min(leftWidth, newPos));
  offset.value = newPos;
  startClientX = e.clientX;
}

function onMouseUp() {
  document.removeEventListener('mousemove', onMouseMove);
  document.removeEventListener('mouseup', onMouseUp);
  snap();
}

function snap() {
  animating = true;
  if (offset.value < -30) {
    currentOffset = -rightWidth;
  } else if (offset.value > 30) {
    currentOffset = leftWidth;
  } else {
    currentOffset = 0;
  }
  offset.value = currentOffset;
}

function close() {
  if (currentOffset !== 0) {
    currentOffset = 0;
    offset.value = 0;
    animating = true;
  }
}

defineExpose({ close });

onUnmounted(() => {
  document.removeEventListener('mousemove', onMouseMove);
  document.removeEventListener('mouseup', onMouseUp);
});
</script>

<style scoped>
.content-layer {
  will-change: transform;
}
.no-transition {
  transition: none !important;
}
</style>
