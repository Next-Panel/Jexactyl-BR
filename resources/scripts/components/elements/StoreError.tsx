import React from 'react';
import { Alert } from '@/components/elements/alert';
import PageContentBlock from '@/components/elements/PageContentBlock';

interface Props {
    message: string;
    admin?: string;
    link?: string;
}

export default ({ message, admin, link }: Props) => (
    <PageContentBlock>
        <Alert type={'error'}>{message}</Alert>
        {admin && (
            <Alert type={'warning'} className={'mt-2'}>
                (Admin Message) {admin}
                <a href={link} className={'text-blue-400 ml-1'} target={'_blank'} rel={'noreferrer'}>
                Como corrigir este erro?
                </a>
            </Alert>
        )}
    </PageContentBlock>
);
