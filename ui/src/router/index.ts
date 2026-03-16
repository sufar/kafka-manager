import { createRouter, createWebHistory } from 'vue-router';
import ModernLayout from '@/layouts/ModernLayout.vue';

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      component: ModernLayout,
      children: [
        {
          path: '',
          redirect: '/dashboard',
        },
        {
          path: 'dashboard',
          name: 'Dashboard',
          component: () => import('@/views/DashboardView.vue'),
        },
        {
          path: 'clusters',
          name: 'Clusters',
          component: () => import('@/views/ClustersView.vue'),
        },
        {
          path: 'topics',
          name: 'Topics',
          component: () => import('@/views/TopicsView.vue'),
        },
        {
          path: 'messages',
          name: 'Messages',
          component: () => import('@/views/MessagesView.vue'),
        },
        {
          path: 'settings',
          name: 'Settings',
          component: () => import('@/views/SettingsView.vue'),
        },
      ],
    },
  ],
});

export default router;
