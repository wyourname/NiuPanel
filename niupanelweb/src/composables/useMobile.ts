import { ref, onMounted, onUnmounted } from 'vue'

export function useMobile(breakpoint = 768) {
  const isMobile = ref(false)

  const checkMobile = () => {
    if (window.innerWidth === 0) return
    isMobile.value = window.innerWidth <= breakpoint
  }

  onMounted(() => {
    checkMobile()
    window.addEventListener('resize', checkMobile)
  })

  onUnmounted(() => {
    window.removeEventListener('resize', checkMobile)
  })

  return {
    isMobile
  }
}
