import React from 'react';
import tw from 'twin.macro';
import { object, string, setLocale } from 'yup';
import { pt } from 'yup-locales';
import styled from 'styled-components/macro';
import { useFlashKey } from '@/plugins/useFlash';
import { Button } from '@/components/elements/button/index';
import { Field, Form, Formik, FormikHelpers } from 'formik';
import Input, { Textarea } from '@/components/elements/Input';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import { createSSHKey, useSSHKeys } from '@/api/account/ssh-keys';
import FormikFieldWrapper from '@/components/elements/FormikFieldWrapper';

interface Values {
    name: string;
    publicKey: string;
}

const CustomTextarea = styled(Textarea)`
    ${tw`h-32`}
`;

export default () => {
    const { clearAndAddHttpError } = useFlashKey('account');
    const { mutate } = useSSHKeys();
    setLocale(pt);

    const submit = (values: Values, { setSubmitting, resetForm }: FormikHelpers<Values>) => {
        clearAndAddHttpError();

        createSSHKey(values.name, values.publicKey)
            .then((key) => {
                resetForm();
                mutate((data) => (data || []).concat(key));
            })
            .catch((error) => clearAndAddHttpError(error))
            .then(() => setSubmitting(false));
    };

    return (
        <>
            <Formik
                onSubmit={submit}
                initialValues={{ name: '', publicKey: '' }}
                validationSchema={object().shape({
                    name: string().required(),
                    publicKey: string().required(),
                })}
            >
                {({ isSubmitting }) => (
                    <Form>
                        <SpinnerOverlay visible={isSubmitting} />
                        <FormikFieldWrapper label={'Nome da Chave SSH'} name={'name'} css={tw`mb-6`}>
                            <Field name={'name'} as={Input} />
                        </FormikFieldWrapper>
                        <FormikFieldWrapper
                            label={'Chave Pública'}
                            name={'publicKey'}
                            description={'Introduza a sua chave SSH pública.'}
                        >
                            <Field name={'publicKey'} as={CustomTextarea} />
                        </FormikFieldWrapper>
                        <div css={tw`flex justify-end mt-6`}>
                            <Button>Salvar</Button>
                        </div>
                    </Form>
                )}
            </Formik>
        </>
    );
};
