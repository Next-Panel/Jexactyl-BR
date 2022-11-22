import tw from 'twin.macro';
import { breakpoint } from '@/theme';
import * as Icon from 'react-feather';
import React, { useState } from 'react';
import useFlash from '@/plugins/useFlash';
import styled from 'styled-components/macro';
import { useStoreState } from '@/state/hooks';
import { Dialog } from '@/components/elements/dialog';
import { Button } from '@/components/elements/button/index';
import purchaseResource from '@/api/store/purchaseResource';
import TitledGreyBox from '@/components/elements/TitledGreyBox';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import PageContentBlock from '@/components/elements/PageContentBlock';

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

const Wrapper = styled.div`
    ${tw`flex flex-row justify-center items-center`};
`;

export default () => {
    const [open, setOpen] = useState(false);
    const [resource, setResource] = useState('');

    const { addFlash, clearFlashes, clearAndAddHttpError } = useFlash();
    const cost = useStoreState((state) => state.storefront.data!.cost);

    const purchase = (resource: string) => {
        clearFlashes('store:resources');

        purchaseResource(resource)
            .then(() => setOpen(false))
            .then(() =>
                addFlash({
                    type: 'success',
                    key: 'store:resources',
                    message: 'O recurso foi adicionado à sua conta.',
                })
            )
            .catch((error) => {
                clearAndAddHttpError({ key: 'store:resources', error });
            });
    };

    return (
        <PageContentBlock title={'Produtos da Loja'} showFlashKey={'store:resources'}>
            <SpinnerOverlay size={'large'} visible={open} />
            <Dialog.Confirm
                open={open}
                onClose={() => setOpen(false)}
                title={'Confirmar a seleção de recursos'}
                confirm={'Continuar'}
                onConfirmed={() => purchase(resource)}
            >
                Tem certeza de que deseja comprar este recurso? Isso tirará créditos da sua conta e adicionará o
                recurso. Esta não é uma transação reversível.
            </Dialog.Confirm>
            <h1 className={'j-left text-5xl'}>Outros Recursos</h1>
            <h3 className={'j-left text-2xl text-neutral-500'}>Compre mais recursos para adicionar ao seu servidor.</h3>
            <Container className={'j-up lg:grid lg:grid-cols-3 my-10'}>
                <TitledGreyBox title={'Comprar CPU'} css={tw`mt-8 sm:mt-0`}>
                    <Wrapper>
                        <Icon.Cpu size={40} />
                        <Button.Success
                            variant={Button.Variants.Secondary}
                            css={tw`ml-4`}
                            onClick={() => {
                                setOpen(true);
                                setResource('cpu');
                            }}
                        >
                            +50% CPU
                        </Button.Success>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Compre CPU para melhorar o desempenho do servidor.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>Custo por 50% de CPU: {cost.cpu} creditos</p>
                </TitledGreyBox>
                <TitledGreyBox title={'Comprar RAM'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <Wrapper>
                        <Icon.PieChart size={40} />
                        <Button.Success
                            variant={Button.Variants.Secondary}
                            css={tw`ml-4`}
                            onClick={() => {
                                setOpen(true);
                                setResource('memory');
                            }}
                        >
                            +1GB RAM
                        </Button.Success>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Compre RAM para melhorar o desempenho do servidor.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Custo por 1GB RAM: {cost.memory} creditos
                    </p>
                </TitledGreyBox>
                <TitledGreyBox title={'Comprar Disco'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <Wrapper>
                        <Icon.HardDrive size={40} />
                        <Button.Success
                            variant={Button.Variants.Secondary}
                            css={tw`ml-4`}
                            onClick={() => {
                                setOpen(true);
                                setResource('disk');
                            }}
                        >
                            +1GB DE DISCO
                        </Button.Success>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Compre espaço para aumentar o armazenamento do
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        servidor.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Custo por 1GB de disco: {cost.disk} creditos
                    </p>
                </TitledGreyBox>
            </Container>
            <Container className={'j-up lg:grid lg:grid-cols-4 my-10'}>
                <TitledGreyBox title={'Comprar Slot para Servidor'} css={tw`mt-8 sm:mt-0`}>
                    <Wrapper>
                        <Icon.Server size={40} />
                        <Button.Success
                            variant={Button.Variants.Secondary}
                            css={tw`ml-4`}
                            onClick={() => {
                                setOpen(true);
                                setResource('slots');
                            }}
                        >
                            +1 slot
                        </Button.Success>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Compre um slot de servidor para
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        implantar um servidor.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>Custo por slot: {cost.slot} creditos</p>
                </TitledGreyBox>
                <TitledGreyBox title={'Comprar Portas'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <Wrapper>
                        <Icon.Share2 size={40} />
                        <Button.Success
                            variant={Button.Variants.Secondary}
                            css={tw`ml-4`}
                            onClick={() => {
                                setOpen(true);
                                setResource('ports');
                            }}
                        >
                            +1 porta
                        </Button.Success>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Compre uma porta para se conectar ao
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        seu servidor.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>Custo por porta: {cost.port} creditos</p>
                </TitledGreyBox>
                <TitledGreyBox title={'Comprar backups'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <Wrapper>
                        <Icon.Archive size={40} />
                        <Button.Success
                            variant={Button.Variants.Secondary}
                            css={tw`ml-4`}
                            onClick={() => {
                                setOpen(true);
                                setResource('backups');
                            }}
                        >
                            +1 backup
                        </Button.Success>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Compre um backup para proteger seus
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>    
                        dados.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Custo por backup slot: {cost.backup} creditos
                    </p>
                </TitledGreyBox>
                <TitledGreyBox title={'Comprar Banco de Dados'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <Wrapper>
                        <Icon.Database size={40} />
                        <Button.Success
                            variant={Button.Variants.Secondary}
                            css={tw`ml-4`}
                            onClick={() => {
                                setOpen(true);
                                setResource('databases');
                            }}
                        >
                            +1 Banco de Dados
                        </Button.Success>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Compre um Banco de Dados para
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        armazenar dados.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Custo por Banco de Dados: {cost.database} creditos
                    </p>
                </TitledGreyBox>
            </Container>
        </PageContentBlock>
    );
};
