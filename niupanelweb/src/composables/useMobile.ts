import { ref, onMounted, onUnmounted } from 'vue'
import { MOBILE_MAX_WIDTH } from '@/constants/responsive'

export function useMobile(breakpoint = MOBILE_MAX_WIDTH) {
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
