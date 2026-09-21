import { ref } from 'vue'

// Global refresh trigger — any view that modifies data bumps this counter,
// other views watch it and reload their data.
const dataVersion = ref(0)

export function useDataSync() {
  function notifyDataChange() {
    dataVersion.value++
  }

  return { dataVersion, notifyDataChange }
}
