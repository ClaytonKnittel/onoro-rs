import React from 'react';
import { createRoot } from 'react-dom/client';
import { Helmet } from 'react-helmet';

import { App } from 'client/App';
import 'client/styles/global_styles.css';

const APP_TITLE = 'Onoro';
const APP_DESCRIPTION = 'Onoro web client';

const root = createRoot(document.getElementById('root') as HTMLElement);

root.render(
  <React.StrictMode>
    <Helmet>
      <title>{APP_TITLE}</title>
      <meta name='description' content={APP_DESCRIPTION} />
      <link
        rel='stylesheet'
        href='https://fonts.googleapis.com/css?family=Roboto:300,400,500,700&display=swap'
      />
      <meta name='viewport' content='initial-scale=1, width=device-width' />
    </Helmet>
    <App />
  </React.StrictMode>
);
