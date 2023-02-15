import tw from 'twin.macro';
import classNames from 'classnames';
import * as Icon from 'react-feather';
import styled from 'styled-components/macro';
import { megabytesToHuman } from '@/helpers';
import React, { useState, useEffect } from 'react';
import Spinner from '@/components/elements/Spinner';
import ContentBox from '@/components/elements/ContentBox';
import Tooltip from '@/components/elements/tooltip/Tooltip';
import StoreContainer from '@/components/elements/StoreContainer';
import { getResources, Resources } from '@/api/store/getResources';

const Wrapper = styled.div`
    ${tw`text-2xl flex flex-row justify-center items-center`};
`;

interface RowProps {
    className?: string;
    titles?: boolean;
}

interface BoxProps {
    title: string;
    description: string;
    icon: React.ReactElement;
    amount: number;
    toHuman?: boolean;
    suffix?: string;
}

export default ({ className, titles }: RowProps) => {
    const [resources, setResources] = useState<Resources>();

    useEffect(() => {
        getResources().then((resources) => setResources(resources));
    }, []);

    if (!resources) return <Spinner size={'large'} centered />;

    const ResourceBox = (props: BoxProps) => (
        <ContentBox title={titles ? props.title : undefined}>
            <Tooltip content={props.description}>
                <Wrapper>
                    {props.icon}
                    <span className={'ml-2'}>
                        {props.toHuman ? <span className={'sm'}>{megabytesToHuman(props.amount)}</span> : props.amount}
                    </span>
                    {props.suffix}
                </Wrapper>
            </Tooltip>
        </ContentBox>
    );

    return (
        <StoreContainer className={classNames(className, 'j-right grid grid-cols-2 sm:grid-cols-7 gap-x-6 gap-y-2')}>
            <ResourceBox
                title={'Créditos'}
                description={'A quantidade de créditos que você tem disponível.'}
                icon={<Icon.DollarSign />}
                amount={resources.balance}
            />
            <ResourceBox
                title={'CPU'}
                description={'A quantidade de CPU (em %) que você tem disponível.'}
                icon={<Icon.Cpu />}
                amount={resources.cpu}
                suffix={'%'}
            />
            <ResourceBox
                title={'Memória'}
                description={'A quantidade de RAM (em MB/GB) que você tem disponível.'}
                icon={<Icon.PieChart />}
                amount={resources.memory}
                toHuman
            />
            <ResourceBox
                title={'Disco'}
                description={'A quantidade de armazenamento (em MB/GB) que você tem disponível.'}
                icon={<Icon.HardDrive />}
                amount={resources.disk}
                toHuman
            />
            <ResourceBox
                title={'Slots'}
                description={'A quantidade de servidores que você pode implantar.'}
                icon={<Icon.Server />}
                amount={resources.slots}
            />
            <ResourceBox
                title={'Portas'}
                description={'A quantidade de portas que você pode adicionar aos seus servidores.'}
                icon={<Icon.Share2 />}
                amount={resources.ports}
            />
            <ResourceBox
                title={'Backups'}
                description={'A quantidade de slots de backup que você pode adicionar aos seus servidores.'}
                icon={<Icon.Archive />}
                amount={resources.backups}
            />
            <ResourceBox
                title={'Databases'}
                description={'A quantidade de slots de Database que você pode adicionar aos seus servidores.'}
                icon={<Icon.Database />}
                amount={resources.databases}
            />
        </StoreContainer>
    );
};
