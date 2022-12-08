import tw from 'twin.macro';
import { breakpoint } from '@/theme';
import * as Icon from 'react-feather';
import { Link } from 'react-router-dom';
import useFlash from '@/plugins/useFlash';
import styled from 'styled-components/macro';
import React, { useState, useEffect } from 'react';
import Spinner from '@/components/elements/Spinner';
import { Button } from '@/components/elements/button';
import { Dialog } from '@/components/elements/dialog';
import { getCosts, Costs } from '@/api/store/getCosts';
import purchaseResource from '@/api/store/purchaseResource';
import TitledGreyBox from '@/components/elements/TitledGreyBox';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import PurchaseBox from '@/components/elements/store/PurchaseBox';
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

export default () => {
    const [open, setOpen] = useState(false);
    const [costs, setCosts] = useState<Costs>();
    const [resource, setResource] = useState('');
    const { addFlash, clearFlashes, clearAndAddHttpError } = useFlash();

    useEffect(() => {
        getCosts().then((costs) => setCosts(costs));
    }, []);

    const purchase = (resource: string) => {
        clearFlashes('store:resources');

        purchaseResource(resource)
            .then(() => {
                setOpen(false);
                addFlash({
                    type: 'success',
                    key: 'store:resources',
                    message: 'O recurso foi adicionado à sua conta.',
                });
            })
            .catch((error) => clearAndAddHttpError({ key: 'store:resources', error }));
    };

    if (!costs) return <Spinner size={'large'} centered />;

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
                Tem certeza de que deseja comprar este recurso ({resource})? Isso gastará os créditos da sua conta e
                adicionará o recurso. Esta não é uma transação reversível.
            </Dialog.Confirm>
            <div className={'my-10'}>
                <Link to={'/store'}>
                    <Button.Text className={'w-full lg:w-1/6 m-2'}>
                        <Icon.ArrowLeft className={'mr-1'} />
                        Voltar à Loja
                    </Button.Text>
                </Link>
            </div>
            <h1 className={'j-left text-5xl'}>Ordenar recursos</h1>
            <h3 className={'j-left text-2xl text-neutral-500'}>Compre mais recursos para adicionar ao seu servidor.</h3>
            <Container className={'j-up lg:grid lg:grid-cols-4 my-10 gap-8'}>
                <PurchaseBox
                    type={'CPU'}
                    amount={50}
                    suffix={'%'}
                    cost={costs.cpu}
                    setOpen={setOpen}
                    icon={<Icon.Cpu />}
                    setResource={setResource}
                    description={'Compre CPU para melhorar os tempos de carregamento e o desempenho do servidor.'}
                />
                <PurchaseBox
                    type={'Memory'}
                    amount={1}
                    suffix={'GB'}
                    cost={costs.memory}
                    setOpen={setOpen}
                    icon={<Icon.PieChart />}
                    setResource={setResource}
                    description={'Compre RAM para melhorar o desempenho geral do servidor.'}
                />
                <PurchaseBox
                    type={'Disk'}
                    amount={1}
                    suffix={'GB'}
                    cost={costs.disk}
                    setOpen={setOpen}
                    icon={<Icon.HardDrive />}
                    setResource={setResource}
                    description={'Comprar disco para armazenar mais arquivos.'}
                />
                <PurchaseBox
                    type={'Slots'}
                    amount={1}
                    cost={costs.slots}
                    setOpen={setOpen}
                    icon={<Icon.Server />}
                    setResource={setResource}
                    description={'Comprar um slot de servidor para que você possa implantar um novo servidor.'}
                />
            </Container>
            <Container className={'j-up lg:grid lg:grid-cols-4 my-10 gap-8'}>
                <PurchaseBox
                    type={'Ports'}
                    amount={1}
                    cost={costs.ports}
                    setOpen={setOpen}
                    icon={<Icon.Share2 />}
                    setResource={setResource}
                    description={'Compre uma porta de rede para adicionar a um servidor.'}
                />
                <PurchaseBox
                    type={'Backups'}
                    amount={1}
                    cost={costs.backups}
                    setOpen={setOpen}
                    icon={<Icon.Archive />}
                    setResource={setResource}
                    description={'Compre um backup para manter seus dados seguros.'}
                />
                <PurchaseBox
                    type={'Databases'}
                    amount={1}
                    cost={costs.databases}
                    setOpen={setOpen}
                    icon={<Icon.Database />}
                    setResource={setResource}
                    description={'Compre um banco de dados para obter e definir dados.'}
                />
                <TitledGreyBox title={'Como usar os recursos'}>
                    <p className={'font-semibold'}>Adicionando a um servidor existente</p>
                    <p className={'text-xs text-gray-500'}>
                        ISe você tiver um servidor que já está implantado, você pode adicionar recursos a ele indo para
                        a guia editar.
                    </p>
                    <p className={'font-semibold mt-1'}>Adicionando a um novo servidor</p>
                    <p className={'text-xs text-gray-500'}>
                        Você pode comprar recursos e adicioná-los a um novo servidor na página de criação do servidor,
                        que você pode acessar através da loja.
                    </p>
                </TitledGreyBox>
            </Container>
            <div className={'flex justify-center items-center'}>
                <div className={'bg-auto bg-center bg-storeone p-4 m-4 rounded-lg'}>
                    <div className={'text-center bg-gray-900 bg-opacity-75 p-4'}>
                        <h1 className={'j-down text-4xl'}>Pronto para começar?</h1>
                        <Link to={'/store/create'}>
                            <Button.Text className={'j-up w-full mt-4'}>Criar o Servidor</Button.Text>
                        </Link>
                    </div>
                </div>
            </div>
        </PageContentBlock>
    );
};
