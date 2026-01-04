<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";

const getUser = async () => {
  const user = await invoke("get_user");
  console.log(user);
};
</script>

<template>
  <button @click="getUser()">Get User</button>
  
</template>

<style scoped>

</style>
