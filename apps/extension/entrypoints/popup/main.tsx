import {createRoot} from 'react-dom/client';
import {ToastProvider} from '@/components/lime/ui/toast';
import {LimePortalProvider} from '@/lib/lime/portal';
import {App} from './App';
import './style.css';

if (matchMedia('(prefers-color-scheme: dark)').matches) document.documentElement.dataset.theme = 'dark';

createRoot(document.getElementById('root')!).render(
  <LimePortalProvider>
    <ToastProvider contained>
      <App/>
    </ToastProvider>
  </LimePortalProvider>,
);
