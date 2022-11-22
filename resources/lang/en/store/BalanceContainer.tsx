import tw from 'twin.macro';
import { breakpoint } from '@/theme';
import styled from 'styled-components/macro';
import { useStoreState } from '@/state/hooks';
import React, { useEffect, useState } from 'react';
import Spinner from '@/components/elements/Spinner';
import ContentBox from '@/components/elements/ContentBox';
import { getResources, Resources } from '@/api/store/getResources';
import PageContentBlock from '@/components/elements/PageContentBlock';
import StripePurchaseForm from '@/components/store/forms/StripePurchaseForm';
import PaypalPurchaseForm from '@/components/store/forms/PaypalPurchaseForm';

const Container = styled.div`
    ${tw`flex flex-wrap`};

    & > div {
        ${tw`w-full`};

        ${breakpoint('sm')`
      width: calc(50% - 1rem);
    `}

        ${breakpoint('md')`
      ${tw`w-auto flex-1`};
    `}
    }
`;

export default () => {
    const [resources, setResources] = useState<Resources>();
    const earn = useStoreState((state) => state.storefront.data!.earn);
    const paypal = useStoreState((state) => state.storefront.data!.gateways?.paypal);
    const stripe = useStoreState((state) => state.storefront.data!.gateways?.stripe);

    useEffect(() => {
        getResources().then((resources) => setResources(resources));
    }, []);

    if (!resources) return <Spinner size={'large'} centered />;

    return (
        <PageContentBlock title={'Carteira'}>
            <h1 className={'j-left text-5xl'}>Carteira</h1>
            <h3 className={'j-left text-2xl mt-2 text-neutral-500'}>Compre créditos facilmente via Stripe ou PayPal.</h3>
            <Container className={'j-up lg:grid lg:grid-cols-2 my-10'}>
                <ContentBox title={'Saldo da Conta'} showFlashes={'account:balance'} css={tw`sm:mt-0`}>
                    <h1 css={tw`text-7xl flex justify-center items-center`}>
                        {resources.balance} <span className={'text-base ml-4'}>creditos</span>
                    </h1>
                </ContentBox>
                <ContentBox title={'Comprar Creditos'} showFlashes={'account:balance'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    {paypal && <PaypalPurchaseForm />}
                    {stripe && <StripePurchaseForm />}
                    <p className={'text-gray-400 text-sm m-2'}>
                        Se nenhum gateway aparecer aqui, é porque eles ainda não foram configurados.
                    </p>
                </ContentBox>
            </Container>
            {earn.enabled && (
                <>
                    <h1 className={'j-left text-5xl'}>Ganhos de crédito Ociosos</h1>
                    <h3 className={'j-left text-2xl text-neutral-500'}>
                        Veja quantos créditos você receberá por minuto de AFK.
                    </h3>
                    <Container className={'j-up lg:grid lg:grid-cols-2 my-10'}>
                        <ContentBox title={'Earn Rate'} showFlashes={'earn:rate'} css={tw`sm:mt-0`}>
                            <h1 css={tw`text-7xl flex justify-center items-center`}>
                                {earn.amount} <span className={'text-base ml-4'}>creditos / min</span>
                            </h1>
                        </ContentBox>
                        <ContentBox title={'Como ganhar'} showFlashes={'earn:how'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                            <p>Você pode ganhar créditos tendo qualquer página deste painel aberta.</p>
                            <p css={tw`mt-1`}>
                                <span css={tw`text-green-500`}>{earn.amount}&nbsp;</span>
                                credito(s) por minuto será automaticamente adicionado à sua conta, desde que este site
                                esteja aberto em uma guia do navegador.
                            </p>
                        </ContentBox>
                    </Container>
                </>
            )}
        </PageContentBlock>
    );
};
