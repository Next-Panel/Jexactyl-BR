import React from 'react';
import { useStoreState } from 'easy-peasy';
import useWindowDimensions from '@/plugins/useWindowDimensions';
import ResourceBar from '@/components/elements/store/ResourceBar';
import StoreBanner from '@/components/elements/store/StoreBanner';
import PageContentBlock from '@/components/elements/PageContentBlock';

export default () => {
    const { width } = useWindowDimensions();
    const username = useStoreState((state) => state.user.data!.username);

    return (
        <PageContentBlock title={'Visão geral da Loja'}>
            <div className={'flex flex-row items-center justify-between mt-10'}>
                {width >= 1280 && (
                    <div>
                        <h1 className={'j-left text-6xl'}>Opa, {username}!</h1>
                        <h3 className={'j-left text-2xl mt-2 text-neutral-500'}>👋 Bem-vindo à loja.</h3>
                    </div>
                )}
                <ResourceBar className={'w-full lg:w-3/4'} />
            </div>
            <div className={'lg:grid lg:grid-cols-3 gap-8 my-10'}>
                <StoreBanner
                    title={'Deseja criar um servidor?'}
                    className={'bg-storeone'}
                    action={'Criar'}
                    link={'create'}
                />
                <StoreBanner
                    title={'Precisa de mais recursos?'}
                    className={'bg-storetwo'}
                    action={'Comprar Recursos'}
                    link={'resources'}
                />
                <StoreBanner
                    title={'Ficou sem créditos?'}
                    className={'bg-storethree'}
                    action={'Comprar Créditos'}
                    link={'credits'}
                />
            </div>
        </PageContentBlock>
    );
};
