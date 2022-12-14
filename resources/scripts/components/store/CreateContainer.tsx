import * as Icon from 'react-feather';
import { Form, Formik } from 'formik';
import useFlash from '@/plugins/useFlash';
import { useStoreState } from 'easy-peasy';
import { number, object, string } from 'yup';
import Field from '@/components/elements/Field';
import Select from '@/components/elements/Select';
import { Egg, getEggs } from '@/api/store/getEggs';
import createServer from '@/api/store/createServer';
import Spinner from '@/components/elements/Spinner';
import { getNodes, Node } from '@/api/store/getNodes';
import { getNests, Nest } from '@/api/store/getNests';
import { Button } from '@/components/elements/button';
import InputSpinner from '@/components/elements/InputSpinner';
import StoreError from '@/components/elements/store/StoreError';
import React, { ChangeEvent, useEffect, useState } from 'react';
import TitledGreyBox from '@/components/elements/TitledGreyBox';
import FlashMessageRender from '@/components/FlashMessageRender';
import StoreContainer from '@/components/elements/StoreContainer';
import { getResources, Resources } from '@/api/store/getResources';
import PageContentBlock from '@/components/elements/PageContentBlock';
import {
    faArchive,
    faCube,
    faDatabase,
    faEgg,
    faHdd,
    faLayerGroup,
    faList,
    faMemory,
    faMicrochip,
    faNetworkWired,
    faStickyNote,
} from '@fortawesome/free-solid-svg-icons';

interface CreateValues {
    name: string;
    description: string | null;
    cpu: number;
    memory: number;
    disk: number;
    ports: number;
    backups: number | null;
    databases: number | null;

    egg: number;
    nest: number;
    node: number;
}

export default () => {
    const [loading, setLoading] = useState(false);
    const [resources, setResources] = useState<Resources>();

    const user = useStoreState((state) => state.user.data!);
    const { clearFlashes, clearAndAddHttpError } = useFlash();

    const [egg, setEgg] = useState<number>(0);
    const [eggs, setEggs] = useState<Egg[]>();
    const [nest, setNest] = useState<number>(0);
    const [nests, setNests] = useState<Nest[]>();
    const [node, setNode] = useState<number>(0);
    const [nodes, setNodes] = useState<Node[]>();

    useEffect(() => {
        clearFlashes();

        getResources().then((resources) => setResources(resources));

        getNodes().then((nodes) => {
            setNode(nodes[0].id);
            setNodes(nodes);
        });

        getNests().then((nests) => {
            setNest(nests[0].id);
            setNests(nests);
        });

        getEggs().then((eggs) => {
            setEgg(eggs[0].id);
            setEggs(eggs);
        });
    }, []);

    const changeNest = (e: ChangeEvent<HTMLSelectElement>) => {
        setNest(parseInt(e.target.value));

        getEggs(parseInt(e.target.value)).then((eggs) => {
            setEggs(eggs);
            setEgg(eggs[0].id);
        });
    };

    const submit = (values: CreateValues) => {
        setLoading(true);
        clearFlashes('store:create');

        createServer(values, egg, nest, node)
            .then((data) => {
                if (!data.id) return;

                setLoading(false);
                clearFlashes('store:create');
                // @ts-expect-error this is valid
                window.location = `/server/${data.id}`;
            })
            .catch((error) => {
                setLoading(false);
                clearAndAddHttpError({ key: 'store:create', error });
            });
    };

    if (!resources) return <Spinner size={'large'} centered />;

    if (!nodes) {
        return (
            <StoreError
                message={'Nenhum node está disponível para implantação. Tente novamente mais tarde.'}
                admin={'Verifique se você tem pelo menos um node que possa ser implantado.'}
            />
        );
    }

    if (!nests || !eggs) {
        return (
            <StoreError
                message={'Nenhum tipo de servidor está disponível para implantação. Tente novamente mais tarde.'}
                admin={'Certifique-se de ter pelo menos um egg que esteja em um nest público.'}
            />
        );
    }

    return (
        <PageContentBlock title={'Criar Servidor'} showFlashKey={'store:create'}>
            <Formik
                onSubmit={submit}
                initialValues={{
                    name: `${user.username}'s server`,
                    description: 'Escreva uma descrição do servidor aqui.',
                    cpu: resources.cpu,
                    memory: resources.memory,
                    disk: resources.disk,
                    ports: resources.ports,
                    backups: resources.backups,
                    databases: resources.databases,
                    nest: 1,
                    egg: 1,
                    node: 1,
                }}
                validationSchema={object().shape({
                    name: string().required().min(3),
                    description: string().optional().min(3).max(191),

                    cpu: number().required().min(25).max(resources.cpu),
                    memory: number().required().min(256).max(resources.memory),
                    disk: number().required().min(256).max(resources.disk),

                    ports: number().required().min(1).max(resources.ports),
                    backups: number().optional().max(resources.backups),
                    databases: number().optional().max(resources.databases),

                    node: number().required().default(node),
                    nest: number().required().default(nest),
                    egg: number().required().default(egg),
                })}
            >
                <Form>
                    <h1 className={'j-left text-5xl'}>Detalhes básicos</h1>
                    <h3 className={'j-left text-2xl text-neutral-500'}>
                        Defina os campos básicos para seu novo servidor.
                    </h3>
                    <StoreContainer className={'lg:grid lg:grid-cols-2 my-10 gap-4'}>
                        <TitledGreyBox title={'Nome do Servidor'} icon={faStickyNote} className={'mt-8 sm:mt-0'}>
                            <Field name={'name'} />
                            <p className={'mt-1 text-xs'}>Atribuir um nome ao servidor para uso no Painel.</p>
                            <p className={'mt-1 text-xs text-gray-400'}>
                                Limites de caracteres: <code>a-z A-Z 0-9 _ -</code> . e <code>[Espaço]</code>.
                            </p>
                        </TitledGreyBox>
                        <TitledGreyBox title={'Descrição do Servidor'} icon={faList} className={'mt-8 sm:mt-0'}>
                            <Field name={'description'} />
                            <p className={'mt-1 text-xs'}>Defina uma descrição para o servidor.</p>
                            <p className={'mt-1 text-xs text-yellow-400'}>* Opcional</p>
                        </TitledGreyBox>
                    </StoreContainer>
                    <h1 className={'j-left text-5xl'}>Limites de recursos</h1>
                    <h3 className={'j-left text-2xl text-neutral-500'}>
                        Defina limites específicos para CPU, RAM e muito mais.
                    </h3>
                    <StoreContainer className={'lg:grid lg:grid-cols-3 my-10 gap-4'}>
                        <TitledGreyBox
                            title={'Limite de CPU do servidor'}
                            icon={faMicrochip}
                            className={'mt-8 sm:mt-0'}
                        >
                            <Field name={'cpu'} />
                            <p className={'mt-1 text-xs'}>Atribua um limite para CPU utilizável.</p>
                            <p className={'mt-1 text-xs text-gray-400'}>{resources.cpu}% disponível</p>
                        </TitledGreyBox>
                        <TitledGreyBox title={'Limite de RAM do servidor'} icon={faMemory} className={'mt-8 sm:mt-0'}>
                            <div className={'relative'}>
                                <Field name={'memory'} />
                                <p className={'absolute text-sm top-1.5 right-2 bg-gray-700 p-2 rounded-lg'}>MB</p>
                            </div>
                            <p className={'mt-1 text-xs'}>Atribuir um limite para RAM utilizável.</p>
                            <p className={'mt-1 text-xs text-gray-400'}>{resources.memory}MB disponível</p>
                        </TitledGreyBox>
                        <TitledGreyBox
                            title={'Limite de armazenamento do servidor'}
                            icon={faHdd}
                            className={'mt-8 sm:mt-0'}
                        >
                            <div className={'relative'}>
                                <Field name={'disk'} />
                                <p className={'absolute text-sm top-1.5 right-2 bg-gray-700 p-2 rounded-lg'}>MB</p>
                            </div>
                            <p className={'mt-1 text-xs'}>Atribuir um limite para armazenamento utilizável.</p>
                            <p className={'mt-1 text-xs text-gray-400'}>{resources.disk}MB disponível</p>
                        </TitledGreyBox>
                    </StoreContainer>
                    <h1 className={'j-left text-5xl'}>Limites de recursos</h1>
                    <h3 className={'j-left text-2xl text-neutral-500'}>
                        Adicionar bancos de dados, alocações e portas ao seu servidor.
                    </h3>
                    <StoreContainer className={'lg:grid lg:grid-cols-3 my-10 gap-4'}>
                        <TitledGreyBox title={'Alocações do servidor'} icon={faNetworkWired} className={'mt-8 sm:mt-0'}>
                            <Field name={'ports'} />
                            <p className={'mt-1 text-xs'}>Atribua portas ao seu servidor.</p>
                            <p className={'mt-1 text-xs text-gray-400'}>{resources.ports} disponível</p>
                        </TitledGreyBox>
                        <TitledGreyBox title={'Backups do servidor'} icon={faArchive} className={'mt-8 sm:mt-0'}>
                            <Field name={'backups'} />
                            <p className={'mt-1 text-xs'}>Atribuia backups ao servidor.</p>
                            <p className={'mt-1 text-xs text-gray-400'}>{resources.backups} disponível</p>
                        </TitledGreyBox>
                        <TitledGreyBox title={'Databases do Servidor'} icon={faDatabase} className={'mt-8 sm:mt-0'}>
                            <Field name={'databases'} />
                            <p className={'mt-1 text-xs'}>Atribuia bancos de dados ao servidor.</p>
                            <p className={'mt-1 text-xs text-gray-400'}>{resources.databases} disponível</p>
                        </TitledGreyBox>
                    </StoreContainer>
                    <h1 className={'j-left text-5xl'}>Implantação</h1>
                    <h3 className={'j-left text-2xl text-neutral-500'}>Escolher um node e um tipo de servidor.</h3>
                    <StoreContainer className={'lg:grid lg:grid-cols-3 my-10 gap-4'}>
                        <TitledGreyBox title={'Nodes disponíveis'} icon={faLayerGroup} className={'mt-8 sm:mt-0'}>
                            <Select name={'node'} onChange={(e) => setNode(parseInt(e.target.value))}>
                                {nodes.map((n) => (
                                    <option key={n.id} value={n.id}>
                                        {n.name} - {n.fqdn} | {100 - parseInt(((n?.used / n?.total) * 100).toFixed(0))}%
                                        espaço restante
                                    </option>
                                ))}
                            </Select>
                            <p className={'mt-1 text-xs text-gray-400'}>Selecione um node para implantar o servidor.</p>
                        </TitledGreyBox>
                        <TitledGreyBox title={'Nest do Servidor'} icon={faCube} className={'mt-8 sm:mt-0'}>
                            <Select name={'nest'} onChange={(nest) => changeNest(nest)}>
                                {nests.map((n) => (
                                    <option key={n.id} value={n.id}>
                                        {n.name}
                                    </option>
                                ))}
                            </Select>
                            <p className={'mt-1 text-xs text-gray-400'}>
                                Selecione um nest a ser usado para o servidor.
                            </p>
                        </TitledGreyBox>
                        <TitledGreyBox title={'Egg do Servidor'} icon={faEgg} className={'mt-8 sm:mt-0'}>
                            <Select name={'egg'} onChange={(e) => setEgg(parseInt(e.target.value))}>
                                {eggs.map((e) => (
                                    <option key={e.id} value={e.id}>
                                        {e.name}
                                    </option>
                                ))}
                            </Select>
                            <p className={'mt-1 text-xs text-gray-400'}>
                                Escolha o jogo que você deseja executar no seu servidor.
                            </p>
                        </TitledGreyBox>
                    </StoreContainer>
                    <InputSpinner visible={loading}>
                        <FlashMessageRender byKey={'store:create'} className={'my-2'} />
                        <div className={'text-right'}>
                            <Button
                                type={'submit'}
                                className={'w-1/6 mb-4'}
                                size={Button.Sizes.Large}
                                disabled={loading}
                            >
                                <Icon.Server className={'mr-2'} /> Criar
                            </Button>
                        </div>
                    </InputSpinner>
                </Form>
            </Formik>
        </PageContentBlock>
    );
};
