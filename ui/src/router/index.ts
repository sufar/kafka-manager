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
          path: 'consumer-groups',
          name: 'ConsumerGroups',
          component: () => import('@/views/ConsumerGroupsView.vue'),
        },
        {
          path: 'messages',
          name: 'Messages',
          component: () => import('@/views/MessagesView.vue'),
        },
        {
          path: 'schema-registry',
          name: 'SchemaRegistry',
          component: () => import('@/views/SchemaRegistryView.vue'),
        },
        {
          path: 'users',
          name: 'Users',
          component: () => import('@/views/UsersView.vue'),
        },
        {
          path: 'notifications',
          name: 'Notifications',
          component: () => import('@/views/NotificationsView.vue'),
        },
        {
          path: 'consumer-lag',
          name: 'ConsumerLag',
          component: () => import('@/views/ConsumerLagView.vue'),
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
