import tw from 'twin.macro';
import React, { useState } from 'react';
import { Link } from 'react-router-dom';
import { object, ref, string } from 'yup';
import { ApplicationStore } from '@/state';
import { httpErrorToHuman } from '@/api/http';
import { Formik, FormikHelpers } from 'formik';
import Field from '@/components/elements/Field';
import Input from '@/components/elements/Input';
import { RouteComponentProps } from 'react-router';
import { Actions, useStoreActions } from 'easy-peasy';
import { Button } from '@/components/elements/button/index';
import performPasswordReset from '@/api/auth/performPasswordReset';
import LoginFormContainer from '@/components/auth/LoginFormContainer';

interface Values {
    password: string;
    passwordConfirmation: string;
}

export default ({ match, location }: RouteComponentProps<{ token: string }>) => {
    const [email, setEmail] = useState('');

    const { clearFlashes, addFlash } = useStoreActions((actions: Actions<ApplicationStore>) => actions.flashes);

    const parsed = new URLSearchParams(location.search);
    if (email.length === 0 && parsed.get('email')) {
        setEmail(parsed.get('email') || '');
    }

    const submit = ({ password, passwordConfirmation }: Values, { setSubmitting }: FormikHelpers<Values>) => {
        clearFlashes();
        performPasswordReset(email, {
            token: match.params.token,
            password,
            passwordConfirmation,
        })
            .then(() => {
                // @ts-expect-error this is valid
                window.location = '/';
            })
            .catch((error) => {
                console.error(error);

                setSubmitting(false);
                addFlash({
                    type: 'danger',
                    title: 'Error',
                    message: httpErrorToHuman(error),
                });
            });
    };

    return (
        <Formik
            onSubmit={submit}
            initialValues={{
                password: '',
                passwordConfirmation: '',
            }}
            validationSchema={object().shape({
                password: string()
                    .required('É necessária uma nova palavra-passe.')
                    .min(8, 'A sua nova senha deve ter pelo menos 8 caracteres.'),
                passwordConfirmation: string()
                    .required('A sua nova senha não corresponde.')
                    // @ts-expect-error this is valid
                    .oneOf([ref('password'), null], 'A sua nova senha não corresponde.'),
            })}
        >
            {({ isSubmitting }) => (
                <LoginFormContainer title={'Redefinir Senha'} css={tw`w-full flex`}>
                    <div>
                        <label>E-mail</label>
                        <Input value={email} isLight disabled />
                    </div>
                    <div css={tw`mt-6`}>
                        <Field
                            light
                            label={'Nova Senha'}
                            name={'password'}
                            type={'password'}
                            description={'As senha devem ter pelo menos 8 caracteres de comprimento.'}
                        />
                    </div>
                    <div css={tw`mt-6`}>
                        <Field light label={'Confirmar Nova Senha'} name={'passwordConfirmation'} type={'password'} />
                    </div>
                    <div css={tw`mt-6`}>
                        <Button size={Button.Sizes.Large} css={tw`w-full`} type={'submit'} disabled={isSubmitting}>
                            Resetar Senha
                        </Button>
                    </div>
                    <div css={tw`mt-6 text-center`}>
                        <Link
                            to={'/auth/login'}
                            css={tw`text-xs text-neutral-500 tracking-wide no-underline uppercase hover:text-neutral-600`}
                        >
                            Voltar ao login
                        </Link>
                    </div>
                </LoginFormContainer>
            )}
        </Formik>
    );
};
